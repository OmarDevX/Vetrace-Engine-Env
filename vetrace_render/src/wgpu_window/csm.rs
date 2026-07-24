use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn cascade_split_distances(near: f32, far: f32, cascade_count: usize) -> [f32; SHADOW_CASCADE_COUNT] {
    let mut splits = [far; SHADOW_CASCADE_COUNT];
    let count = cascade_count.clamp(1, SHADOW_CASCADE_COUNT);
    let lambda = 0.65_f32;
    for i in 1..=count {
        let p = i as f32 / count as f32;
        let log = near * (far / near.max(0.001)).powf(p);
        let linear = near + (far - near) * p;
        splits[i - 1] = lambda * log + (1.0 - lambda) * linear;
    }
    for i in count..SHADOW_CASCADE_COUNT {
        splits[i] = far;
    }
    splits
}

pub(super) fn directional_cascade_matrix(
    frame: &RenderFrame,
    light_dir: Vec3,
    shadow_candidates: &[ShadowCandidate],
    split_near: f32,
    split_far: f32,
    shadow_map_size: u32,
) -> Mat4 {
    let aspect = frame.settings.width.max(1) as f32 / frame.settings.height.max(1) as f32;
    let corners = camera_frustum_corners(&frame.camera, aspect, split_near, split_far);
    let mut center = Vec3::ZERO;
    for corner in corners {
        center += corner;
    }
    center /= corners.len() as f32;

    // Stable CSMs should not constantly resize the orthographic XY box.  Fit the
    // camera split with a sphere, quantize that sphere very gently, and use a
    // symmetric [-radius, radius] projection.  This trades a tiny amount of texel
    // density for a shadow projection that does not pulse as frustum corners or
    // nearby caster bounds change by tiny amounts.
    let mut radius = 0.0_f32;
    for corner in corners {
        radius = radius.max(corner.distance(center));
    }
    radius = radius.max(2.0);
    radius = (radius * 16.0).ceil() / 16.0;

    let mut up = Vec3::Y;
    if light_dir.cross(up).length_squared() < 0.0001 {
        up = Vec3::Z;
    }

    let map_size = shadow_map_size.max(1) as f32;
    let texel_size = (radius * 2.0 / map_size).max(0.0001);

    // Snap the cascade center in a light-space basis whose orientation depends
    // only on the light direction, not on the camera center.  Snapping the center,
    // rather than only snapping min/max extents, is what removes the repeating
    // crawling/jiggling when the camera or player moves slowly.
    let light_basis = Mat4::look_at_rh(-light_dir, Vec3::ZERO, up);
    let mut center_ls = light_basis.transform_point3(center);
    center_ls.x = (center_ls.x / texel_size).round() * texel_size;
    center_ls.y = (center_ls.y / texel_size).round() * texel_size;
    let snapped_center = light_basis.inverse().transform_point3(center_ls);

    let view = Mat4::look_at_rh(snapped_center - light_dir * radius * 3.0, snapped_center, up);

    // Keep XY stable.  Caster bounds are allowed to expand only the light-space
    // depth range, otherwise objects entering/leaving the loose caster set make
    // the shadow texel grid resize and visibly pop/jiggle.
    let padding_z = (radius * 0.20).max(4.0);
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    for point in corners {
        let p = view.transform_point3(point);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
    }

    let caster_reach = radius + 40.0;
    for candidate in shadow_candidates {
        let bounds_center = (candidate.bounds_min + candidate.bounds_max) * 0.5;
        let bounds_radius = (candidate.bounds_max - candidate.bounds_min).length() * 0.5;
        if bounds_center.distance(center) > caster_reach + bounds_radius {
            continue;
        }
        for point in bbox_corners(candidate.bounds_min, candidate.bounds_max) {
            let p = view.transform_point3(point);
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
        }
    }

    if !(min_z.is_finite() && max_z.is_finite()) {
        min_z = -radius * 4.0;
        max_z = radius * 4.0;
    }

    // In a RH light view, visible points are in front of the light camera on -Z.
    // Quantize near/far coarsely so depth precision does not shift every frame.
    let depth_snap = (texel_size * 16.0).max(0.05);
    let near = (((-max_z - padding_z).max(0.05)) / depth_snap).floor() * depth_snap;
    let far = (((-min_z + padding_z).max(near + 1.0)) / depth_snap).ceil() * depth_snap;
    let projection = Mat4::orthographic_rh(-radius, radius, -radius, radius, near.max(0.05), far.max(near + 1.0));
    projection * view
}

pub(super) fn camera_frustum_corners(camera: &crate::resources::Camera, aspect: f32, near: f32, far: f32) -> [Vec3; 8] {
    let forward = (camera.target - camera.position).normalize_or_zero();
    let forward = if forward.length_squared() > 0.0 { forward } else { Vec3::NEG_Z };
    let mut right = forward.cross(camera.up).normalize_or_zero();
    if right.length_squared() == 0.0 {
        right = Vec3::X;
    }
    let up = right.cross(forward).normalize_or_zero();
    let tan = (camera.fov_y_radians * 0.5).tan();
    let near_h = tan * near;
    let near_w = near_h * aspect;
    let far_h = tan * far;
    let far_w = far_h * aspect;
    let near_center = camera.position + forward * near;
    let far_center = camera.position + forward * far;
    [
        near_center - right * near_w - up * near_h,
        near_center + right * near_w - up * near_h,
        near_center + right * near_w + up * near_h,
        near_center - right * near_w + up * near_h,
        far_center - right * far_w - up * far_h,
        far_center + right * far_w - up * far_h,
        far_center + right * far_w + up * far_h,
        far_center - right * far_w + up * far_h,
    ]
}

pub(super) fn bbox_corners(min: Vec3, max: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]
}

pub(super) fn primary_shadow_light(frame: &RenderFrame) -> Option<(Vec3, ShadowMode)> {
    // Shadows are strictly opt-in. Lighting may use renderer fallback lights,
    // but shadow maps are only rendered when an actual scene directional light
    // explicitly asks for ShadowMode::Hard or ShadowMode::Soft.
    frame
        .directional_lights
        .iter()
        .find(|light| light.shadow_mode != ShadowMode::None)
        .map(|light| (light.direction, light.shadow_mode))
}
