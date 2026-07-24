use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn material_uniform_from_material(material: &Material, custom: Option<&CustomShaderMaterial>, frame: &RenderFrame) -> CustomShaderUniform {
    let mut uniform = if let Some(custom) = custom {
        CustomShaderUniform::from_material(custom, &frame.settings)
    } else {
        let temp = CustomShaderMaterial {
            shader_id: "__material".to_string(),
            asset_path: None,
            wgsl_source: None,
            params: vec![0.0, 1.0],
            fallback_color_a: material.base_color,
            fallback_color_b: material.emissive,
            ..CustomShaderMaterial::default()
        };
        CustomShaderUniform::from_material(&temp, &frame.settings)
    };
    if custom.is_none() {
        let base = material.base_color.clamp(Vec3::ZERO, Vec3::ONE);
        let emissive = material.emissive.max(Vec3::ZERO);
        uniform.color_a = [base.x, base.y, base.z, material.alpha.clamp(0.0, 1.0)];
        uniform.color_b = [emissive.x, emissive.y, emissive.z, 1.0];
    }
    if custom.is_none() {
        let uv_scale = material.uv_scale.max(Vec2::splat(0.0001));
        uniform.params[0][0] = uv_scale.x;
        uniform.params[0][1] = uv_scale.y;
    }
    let flags = if material.is_glass { 1.0 } else { 0.0 };
    uniform.set_pbr(material.roughness, material.metallic, material.alpha, flags);
    uniform.set_pbr_extra(
        material.normal_scale,
        material.occlusion_strength,
        material.alpha_cutoff,
        alpha_mode_code(material.alpha_mode),
    );
    let (direction, color, intensity, ambient) = primary_light_for_frame(frame);
    uniform.set_lighting(direction, color, intensity, ambient);
    set_scene_lights_on_uniform(&mut uniform, frame, ambient);
    set_fog_on_uniform(&mut uniform, frame);
    uniform.set_post_process(
        frame.post_processing.exposure,
        frame.post_processing.gamma,
        frame.post_processing.tone_mapper.shader_value(),
    );
    uniform
}

pub(super) fn set_fog_on_uniform(uniform: &mut CustomShaderUniform, frame: &RenderFrame) {
    if let Some(fog) = frame.fog.as_ref().filter(|fog| fog.enabled && fog.density > 0.0) {
        uniform.set_fog(true, fog.color, fog.density, fog.anisotropy);
    } else {
        uniform.set_fog(false, Vec3::splat(0.6), 0.0, 0.0);
    }
}

pub(super) fn primary_light_for_frame(frame: &RenderFrame) -> (Vec3, Vec3, f32, f32) {
    let gi_enabled = !matches!(
        frame.post_processing.gi_mode,
        crate::components::GlobalIlluminationMode::Off
    );
    if let Some(light) = frame.directional_lights.first() {
        let ambient = if gi_enabled {
            if frame.atmosphere.is_some() { 0.22 } else { 0.28 }
        } else {
            0.0
        };
        (light.direction, light.color, light.intensity, ambient)
    } else {
        let ambient = if gi_enabled { 0.50 } else { 0.0 };
        (Vec3::new(-0.35, -1.0, -0.25), Vec3::ONE, 1.0, ambient)
    }
}

pub(super) fn set_scene_lights_on_uniform(uniform: &mut CustomShaderUniform, frame: &RenderFrame, ambient: f32) {
    let has_real_lights = !frame.directional_lights.is_empty() || !frame.point_lights.is_empty() || !frame.spot_lights.is_empty();

    uniform.directional_lights = [[0.0; 4]; 4];
    uniform.directional_colors = [[0.0; 4]; 4];
    uniform.point_lights = [[0.0; 4]; 8];
    uniform.point_colors_ranges = [[0.0; 4]; 8];
    uniform.spot_lights = [[0.0; 4]; 4];
    uniform.spot_dirs_ranges = [[0.0; 4]; 4];
    uniform.spot_colors_inner = [[0.0; 4]; 4];
    uniform.spot_params = [[0.0; 4]; 4];

    if !has_real_lights {
        let fallback_dir = Vec3::new(-0.35, -1.0, -0.25).normalize();
        uniform.light_counts = [1.0, 0.0, 0.0, ambient.clamp(0.0, 1.0)];
        uniform.directional_lights[0] = [fallback_dir.x, fallback_dir.y, fallback_dir.z, 1.0];
        uniform.directional_colors[0] = [1.0, 1.0, 1.0, 0.0];
        return;
    }

    let directional_count = frame.directional_lights.len().min(MAX_DIRECTIONAL_LIGHTS);
    for (slot, light) in frame.directional_lights.iter().take(MAX_DIRECTIONAL_LIGHTS).enumerate() {
        let direction = light.direction.normalize_or_zero();
        let direction = if direction.length_squared() > 0.0 { direction } else { Vec3::new(0.0, -1.0, 0.0) };
        let color = light.color.max(Vec3::ZERO);
        uniform.directional_lights[slot] = [direction.x, direction.y, direction.z, light.intensity.max(0.0)];
        uniform.directional_colors[slot] = [color.x, color.y, color.z, 0.0];
    }

    let point_count = frame.point_lights.len().min(MAX_POINT_LIGHTS);
    for (slot, light) in frame.point_lights.iter().take(MAX_POINT_LIGHTS).enumerate() {
        let color = light.color.max(Vec3::ZERO);
        let range = light.range.unwrap_or(0.0).max(0.0);
        uniform.point_lights[slot] = [light.position.x, light.position.y, light.position.z, light.intensity.max(0.0)];
        uniform.point_colors_ranges[slot] = [color.x, color.y, color.z, range];
    }

    let spot_count = frame.spot_lights.len().min(MAX_SPOT_LIGHTS);
    for (slot, light) in frame.spot_lights.iter().take(MAX_SPOT_LIGHTS).enumerate() {
        let color = light.color.max(Vec3::ZERO);
        let direction = light.direction.normalize_or_zero();
        let direction = if direction.length_squared() > 0.0 { direction } else { Vec3::new(0.0, 0.0, -1.0) };
        let range = light.range.unwrap_or(0.0).max(0.0);
        let inner = light.inner_cone_angle.max(0.0).cos();
        let outer = light.outer_cone_angle.max(light.inner_cone_angle + 0.001).cos();
        uniform.spot_lights[slot] = [light.position.x, light.position.y, light.position.z, light.intensity.max(0.0)];
        uniform.spot_dirs_ranges[slot] = [direction.x, direction.y, direction.z, range];
        uniform.spot_colors_inner[slot] = [color.x, color.y, color.z, inner];
        uniform.spot_params[slot] = [outer, 0.0, 0.0, 0.0];
    }

    uniform.light_counts = [directional_count as f32, point_count as f32, spot_count as f32, ambient.clamp(0.0, 1.0)];
}

pub(super) fn clear_color_for_frame(frame: &RenderFrame) -> [f32; 4] {
    let mut color = if let Some(atmosphere) = &frame.atmosphere {
        (atmosphere.sky_tint * atmosphere.intensity.max(0.0)).clamp(Vec3::ZERO, Vec3::ONE)
    } else {
        Vec3::new(frame.settings.clear_color[0], frame.settings.clear_color[1], frame.settings.clear_color[2])
    };

    // The object shader applies depth/distance fog to geometry. The clear color
    // also needs a small aerial-perspective tint, otherwise fog appears to do
    // nothing when the camera looks at empty sky/background.
    if let Some(fog) = frame.fog.as_ref().filter(|fog| fog.enabled && fog.density > 0.0) {
        let fog_amount = 1.0 - (-fog.density.max(0.0) * 80.0).exp();
        color = color.lerp(fog.color.clamp(Vec3::ZERO, Vec3::ONE), fog_amount.clamp(0.0, 0.85));
    }
    [color.x, color.y, color.z, frame.settings.clear_color[3]]
}

pub(super) fn camera_matrix(camera: &Camera, width: u32, height: u32) -> Mat4 {
    let view = Mat4::look_at_rh(camera.position, camera.target, camera.up);
    let aspect = width.max(1) as f32 / height.max(1) as f32;
    let projection = Mat4::perspective_rh(camera.fov_y_radians, aspect, camera.near, camera.far);
    projection * view
}

pub(super) fn camera_matrix_for_surface(frame: &RenderFrame, width: u32, height: u32) -> Mat4 {
    camera_matrix(&frame.camera, width, height)
}

pub(super) fn camera_uniform_for(camera: &Camera, width: u32, height: u32) -> CameraUniform {
    let view_proj = camera_matrix(camera, width, height);
    let camera_forward = (camera.target - camera.position).normalize_or_zero();
    let camera_forward = if camera_forward.length_squared() > 0.0 {
        camera_forward
    } else {
        Vec3::NEG_Z
    };
    CameraUniform {
        view_proj: view_proj.to_cols_array_2d(),
        camera_position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
        camera_forward: [camera_forward.x, camera_forward.y, camera_forward.z, 0.0],
        inverse_view_proj: view_proj.inverse().to_cols_array_2d(),
    }
}
