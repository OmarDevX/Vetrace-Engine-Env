use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) fn object_sort_depth(object: &RenderObject, camera_position: Vec3) -> f32 {
    object.transform.translation.distance_squared(camera_position)
}

pub(super) fn build_directional_shadow_candidates(
    pending_draws: &[PendingDraw<'_>],
    shadow_enabled: bool,
    shadow_vertex_limit: usize,
    camera_position: Vec3,
    max_distance: f32,
) -> Vec<ShadowCandidate> {
    if !shadow_enabled || shadow_vertex_limit < 3 {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    for (index, draw) in pending_draws.iter().enumerate() {
        if draw.geometry.draw_count() < 3
            || !casts_directional_shadow(&draw.object.material, &draw.pipeline)
            || !object_within_shadow_distance(draw.object, camera_position, max_distance)
        {
            continue;
        }
        let bounds_min = draw.bounds_min;
        let bounds_max = draw.bounds_max;
        if !bounds_min.x.is_finite() || !bounds_max.x.is_finite() {
            continue;
        }
        candidates.push(ShadowCandidate {
            index,
            priority: shadow_candidate_priority(draw),
            distance2: object_sort_depth(draw.object, camera_position),
            vertices: draw.geometry.draw_count(),
            bounds_min,
            bounds_max,
        });
    }

    candidates.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then_with(|| a.distance2.partial_cmp(&b.distance2).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.vertices.cmp(&b.vertices))
    });

    let mut selected = Vec::new();
    let mut used = 0_usize;
    for mut candidate in candidates {
        if used >= shadow_vertex_limit {
            break;
        }
        let remaining = shadow_vertex_limit.saturating_sub(used);
        let copy_count = (candidate.vertices.min(remaining) / 3) * 3;
        if copy_count == 0 {
            continue;
        }
        candidate.vertices = copy_count;
        used = used.saturating_add(copy_count);
        selected.push(candidate);
    }
    selected
}

pub(super) fn shadow_candidate_priority(draw: &PendingDraw<'_>) -> u8 {
    // Do not let a large imported GLB consume the entire shadow budget before
    // small game-side primitive actors get a chance to cast shadows.  Simple
    // Shooter players/enemies are primitive/custom-shader cubes, while the test
    // car is an imported mesh with many more vertices.
    if draw.object.mesh.is_none() {
        0
    } else if draw.geometry.draw_count() <= 256 {
        1
    } else {
        2
    }
}

pub(super) fn normalize_shadow_map_size(size: u32) -> u32 {
    match size {
        0 => 1,
        1..=512 => 512,
        513..=1024 => 1024,
        1025..=2048 => 2048,
        _ => 4096,
    }
}

pub(super) fn normalize_shadow_cascade_count(count: u32) -> usize {
    count.clamp(1, SHADOW_CASCADE_COUNT as u32) as usize
}

pub(super) fn disabled_shadow_info(settings: &RenderSettings) -> ShadowInfo {
    ShadowInfo {
        enabled: false,
        soft_radius: 0.0,
        view_proj: [Mat4::IDENTITY; SHADOW_CASCADE_COUNT],
        cascade_splits: [10_000.0; SHADOW_CASCADE_COUNT],
        cascade_count: 0,
        bias: settings.shadow_bias.max(0.0),
        slope_bias: settings.shadow_slope_bias.max(0.0),
        normal_bias: settings.shadow_normal_bias.max(0.0),
        pcf_quality: settings.shadow_pcf_quality.max(1) as f32,
        filter_mode: ShadowFilterMode::Hard,
        pcss_light_radius: 0.0,
        evsm_blur_radius: settings.shadow_evsm_blur_radius.max(0.0),
        evsm_exponent: settings.shadow_evsm_exponent.clamp(1.0, 5.5),
    }
}

pub(super) fn object_within_shadow_distance(object: &RenderObject, camera_position: Vec3, max_distance: f32) -> bool {
    if max_distance <= 0.0 {
        return true;
    }
    object.transform.translation.distance_squared(camera_position) <= max_distance * max_distance
}

pub(super) fn casts_directional_shadow(material: &Material, pipeline: &PipelineKind) -> bool {
    if is_transparent_pipeline(pipeline) {
        return false;
    }
    material.alpha_mode != AlphaMode::Blend && material.alpha > 0.001
}

pub(super) fn vertex_bounds(vertices: &[GpuVertex]) -> (Vec3, Vec3) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for vertex in vertices {
        let p = Vec3::from_array(vertex.position);
        min = min.min(p);
        max = max.max(p);
    }
    if !(min.x.is_finite() && min.y.is_finite() && min.z.is_finite() && max.x.is_finite() && max.y.is_finite() && max.z.is_finite()) {
        return (Vec3::splat(f32::NAN), Vec3::splat(f32::NAN));
    }
    (min, max)
}

pub(super) fn world_vertex_bounds(object: &RenderObject, vertices: &[GpuVertex]) -> (Vec3, Vec3) {
    let (local_min, local_max) = vertex_bounds(vertices);
    if !local_min.x.is_finite() || !local_max.x.is_finite() {
        return (Vec3::splat(f32::NAN), Vec3::splat(f32::NAN));
    }
    let model = object_model_matrix(object);
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for point in bbox_corners(local_min, local_max) {
        let p = model.transform_point3(point);
        min = min.min(p);
        max = max.max(p);
    }
    if !(min.x.is_finite() && max.x.is_finite()) {
        return (Vec3::splat(f32::NAN), Vec3::splat(f32::NAN));
    }
    (min, max)
}

pub(super) fn scene_cache_key(entity_id: u64, pass: u64) -> u64 {
    entity_id ^ pass.wrapping_mul(0x9e3779b97f4a7c15)
}

pub(super) fn geometry_cache_key(object: &RenderObject, extra: Option<f32>) -> u64 {
    let extra_bits = extra.unwrap_or(0.0).to_bits() as u64;
    let base = if let Some(mesh) = object.mesh {
        0x6d65736800000000_u64 ^ mesh.0
    } else {
        0x7368617065000000_u64 ^ shape_signature(object.shape.as_ref())
    };
    let scale = outline_scale_signature(object, extra);
    base
        ^ extra_bits.rotate_left(17)
        ^ (scale[0] as u64).rotate_left(7)
        ^ (scale[1] as u64).rotate_left(23)
        ^ (scale[2] as u64).rotate_left(41)
        ^ object.geometry_revision.rotate_left(13)
}

pub(super) fn geometry_buffer_signature(object: &RenderObject, geometry: &IndexedGeometry, extra: Option<f32>) -> GeometryBufferSignature {
    GeometryBufferSignature {
        mesh_id: object.mesh.map(|mesh| mesh.0).unwrap_or(u64::MAX),
        shape_kind: shape_signature(object.shape.as_ref()),
        vertex_count: geometry.vertices.len() as u32,
        index_count: geometry.indices.as_ref().map(|indices| indices.len() as u32).unwrap_or(0),
        extra: extra.unwrap_or(0.0).to_bits(),
        outline_scale: outline_scale_signature(object, extra),
        geometry_revision: object.geometry_revision,
    }
}

pub(super) fn outline_scale_signature(object: &RenderObject, extra: Option<f32>) -> [u32; 3] {
    // Primitive UVs are generated in local geometry using object scale so
    // textures repeat by real size instead of stretching when the editor scales
    // a wall/floor.  Mesh assets keep their authored UVs, so only outline
    // inflation needs a scale-specific buffer for meshes.
    if object.mesh.is_some() && extra.unwrap_or(0.0) <= 0.0 {
        return [0, 0, 0];
    }
    let scale = object.transform.scale.abs().max(Vec3::splat(0.001));
    [
        quantized_scale_bits(scale.x),
        quantized_scale_bits(scale.y),
        quantized_scale_bits(scale.z),
    ]
}

pub(super) fn quantized_scale_bits(value: f32) -> u32 {
    ((if value.is_finite() { value } else { 1.0 }) * 10_000.0).round().to_bits()
}

pub(super) fn material_texture_signature(material: &Material, lightmap_atlas_id: Option<u64>) -> [u64; 6] {
    [
        material.base_color_texture.map(|h| h.0).unwrap_or(u64::MAX),
        material.normal_texture.map(|h| h.0).unwrap_or(u64::MAX),
        material.metallic_roughness_texture.map(|h| h.0).unwrap_or(u64::MAX),
        material.occlusion_texture.map(|h| h.0).unwrap_or(u64::MAX),
        material.emissive_texture.map(|h| h.0).unwrap_or(u64::MAX),
        lightmap_atlas_id.unwrap_or(u64::MAX),
    ]
}

pub(super) fn custom_render_texture_signature(custom: Option<&CustomShaderMaterial>) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    if let Some(custom) = custom {
        for name in custom.render_textures.iter().take(4) {
            for byte in name.as_bytes() {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

pub(super) fn scene_draw_signature(draw: &PendingDraw<'_>, extra: u32) -> SceneDrawSignature {
    outline_scene_draw_signature(
        draw.object,
        &draw.object.material,
        &draw.pipeline,
        draw.geometry.vertices.len(),
        draw.geometry.indices.as_ref().map(|indices| indices.len()).unwrap_or(0),
        draw.object.baked_lightmap.as_ref().map(|lightmap| lightmap.atlas.id),
        extra,
    )
}

pub(super) fn outline_scene_draw_signature(
    object: &RenderObject,
    material: &Material,
    pipeline: &PipelineKind,
    vertex_count: usize,
    index_count: usize,
    lightmap_atlas_id: Option<u64>,
    extra: u32,
) -> SceneDrawSignature {
    SceneDrawSignature {
        mesh_id: object.mesh.map(|mesh| mesh.0).unwrap_or(u64::MAX),
        shape_kind: shape_signature(object.shape.as_ref()),
        vertex_count: vertex_count as u32,
        index_count: index_count as u32,
        material_textures: material_texture_signature(material, lightmap_atlas_id),
        render_textures_hash: custom_render_texture_signature(object.custom_shader.as_ref()),
        pipeline_kind: pipeline_signature(pipeline),
        extra,
    }
}

pub(super) fn shadow_draw_visible_in_cascade(draw: &PreparedShadowDraw, view_proj: Mat4) -> bool {
    let mut any_inside_xy = false;
    let mut all_left = true;
    let mut all_right = true;
    let mut all_below = true;
    let mut all_above = true;
    let mut all_near = true;
    let mut all_far = true;
    for point in bbox_corners(draw.bounds_min, draw.bounds_max) {
        let clip = view_proj * point.extend(1.0);
        if clip.w.abs() < 0.00001 {
            continue;
        }
        let ndc = clip.truncate() / clip.w;
        let pad = 0.20;
        all_left &= ndc.x < -1.0 - pad;
        all_right &= ndc.x > 1.0 + pad;
        all_below &= ndc.y < -1.0 - pad;
        all_above &= ndc.y > 1.0 + pad;
        all_near &= ndc.z < 0.0 - pad;
        all_far &= ndc.z > 1.0 + pad;
        if ndc.x >= -1.0 - pad && ndc.x <= 1.0 + pad && ndc.y >= -1.0 - pad && ndc.y <= 1.0 + pad && ndc.z >= 0.0 - pad && ndc.z <= 1.0 + pad {
            any_inside_xy = true;
        }
    }
    any_inside_xy || !(all_left || all_right || all_below || all_above || all_near || all_far)
}

pub(super) fn shape_signature(shape: Option<&Shape>) -> u64 {
    let Some(shape) = shape else {
        return u64::MAX;
    };
    let kind = match shape.primitive {
        PrimitiveShape::Cube => 1_u64,
        PrimitiveShape::Sphere => 2,
        PrimitiveShape::Capsule => 3,
        PrimitiveShape::Plane => 4,
        PrimitiveShape::Quad => 5,
    };
    let sx = (shape.size.x.to_bits() as u64) & 0xffff;
    let sy = (shape.size.y.to_bits() as u64) & 0xffff;
    let sz = (shape.size.z.to_bits() as u64) & 0xffff;
    kind | (sx << 8) | (sy << 24) | (sz << 40)
}

pub(super) fn pipeline_signature(pipeline: &PipelineKind) -> u64 {
    match pipeline {
        PipelineKind::Default => 1,
        PipelineKind::DefaultDoubleSided => 2,
        PipelineKind::Transparent => 3,
        PipelineKind::TransparentDoubleSided => 4,
        PipelineKind::Custom { key, bucket } => 5 ^ fnv1a64(key.as_bytes()) ^ (custom_bucket_signature(*bucket) << 48),
        PipelineKind::OutlineMask => 6,
        PipelineKind::OutlineOverlay => 7,
    }
}

pub(super) fn custom_bucket_signature(bucket: CustomShaderRenderBucket) -> u64 {
    match bucket {
        CustomShaderRenderBucket::Opaque => 1,
        CustomShaderRenderBucket::Transparent => 2,
        CustomShaderRenderBucket::Overlay => 3,
    }
}

pub(super) fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
