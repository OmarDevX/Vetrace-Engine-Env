use super::*;

pub(super) fn reflection_hash_mix(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(0x100000001b3);
}

pub(super) fn reflection_hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        reflection_hash_mix(hash, *byte as u64);
    }
}

pub(super) fn reflection_hash_vec2(hash: &mut u64, value: Vec2) {
    reflection_hash_mix(hash, value.x.to_bits() as u64);
    reflection_hash_mix(hash, value.y.to_bits() as u64);
}

pub(super) fn reflection_hash_vec3(hash: &mut u64, value: Vec3) {
    reflection_hash_mix(hash, value.x.to_bits() as u64);
    reflection_hash_mix(hash, value.y.to_bits() as u64);
    reflection_hash_mix(hash, value.z.to_bits() as u64);
}

pub(super) fn reflection_object_signature(object: &RenderObject, assets: Option<&RenderAssets>) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    reflection_hash_mix(&mut hash, object.entity.0);
    reflection_hash_vec3(&mut hash, object.transform.translation);
    reflection_hash_mix(&mut hash, object.transform.rotation.x.to_bits() as u64);
    reflection_hash_mix(&mut hash, object.transform.rotation.y.to_bits() as u64);
    reflection_hash_mix(&mut hash, object.transform.rotation.z.to_bits() as u64);
    reflection_hash_mix(&mut hash, object.transform.rotation.w.to_bits() as u64);
    reflection_hash_vec3(&mut hash, object.transform.scale);
    reflection_hash_mix(&mut hash, object.geometry_revision);
    reflection_hash_mix(&mut hash, object.mesh.map_or(u64::MAX, |mesh| mesh.0));
    if let Some(mesh) = object.mesh.and_then(|handle| assets.and_then(|assets| assets.meshes.get(&handle.0))) {
        reflection_hash_mix(&mut hash, mesh.revision);
        reflection_hash_mix(&mut hash, mesh.vertices.len() as u64);
        reflection_hash_mix(&mut hash, mesh.indices.len() as u64);
    }
    if let Some(shape) = object.shape.as_ref() {
        let primitive = match shape.primitive {
            PrimitiveShape::Cube => 1,
            PrimitiveShape::Sphere => 2,
            PrimitiveShape::Capsule => 3,
            PrimitiveShape::Plane => 4,
            PrimitiveShape::Quad => 5,
        };
        reflection_hash_mix(&mut hash, primitive);
        reflection_hash_vec3(&mut hash, shape.size);
    }
    let material = &object.material;
    reflection_hash_vec3(&mut hash, material.base_color);
    reflection_hash_vec3(&mut hash, material.emissive);
    reflection_hash_vec2(&mut hash, material.uv_scale);
    reflection_hash_mix(&mut hash, material.roughness.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.metallic.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.alpha.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.alpha_mode as u64);
    reflection_hash_mix(&mut hash, material.alpha_cutoff.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.double_sided as u64);
    reflection_hash_mix(&mut hash, material.is_glass as u64);
    reflection_hash_vec3(&mut hash, material.specular_f0);
    reflection_hash_mix(&mut hash, material.normal_scale.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.occlusion_strength.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.ior.to_bits() as u64);
    reflection_hash_mix(&mut hash, material.base_color_texture.map_or(u64::MAX, |value| value.0));
    reflection_hash_mix(&mut hash, material.normal_texture.map_or(u64::MAX, |value| value.0));
    reflection_hash_mix(&mut hash, material.metallic_roughness_texture.map_or(u64::MAX, |value| value.0));
    reflection_hash_mix(&mut hash, material.occlusion_texture.map_or(u64::MAX, |value| value.0));
    reflection_hash_mix(&mut hash, material.emissive_texture.map_or(u64::MAX, |value| value.0));
    for handle in [
        material.base_color_texture,
        material.normal_texture,
        material.metallic_roughness_texture,
        material.occlusion_texture,
        material.emissive_texture,
    ].into_iter().flatten() {
        if let Some(texture) = assets.and_then(|assets| assets.textures.get(&handle.0)) {
            reflection_hash_mix(&mut hash, texture.revision);
            reflection_hash_mix(&mut hash, texture.width as u64);
            reflection_hash_mix(&mut hash, texture.height as u64);
            reflection_hash_mix(&mut hash, texture.rgba8.len() as u64);
        }
    }
    if let Some(custom) = object.custom_shader.as_ref() {
        reflection_hash_bytes(&mut hash, custom.shader_id.as_bytes());
        if let Some(path) = custom.asset_path.as_deref() { reflection_hash_bytes(&mut hash, path.as_bytes()); }
        if let Some(source) = custom.wgsl_source.as_deref() { reflection_hash_bytes(&mut hash, source.as_bytes()); }
        reflection_hash_mix(&mut hash, custom.reflection_capture_mode as u64);
        if let Some(path) = custom.reflection_capture_asset_path.as_deref() { reflection_hash_bytes(&mut hash, path.as_bytes()); }
        if let Some(source) = custom.reflection_capture_wgsl_source.as_deref() { reflection_hash_bytes(&mut hash, source.as_bytes()); }
        for value in &custom.params {
            reflection_hash_mix(&mut hash, value.to_bits() as u64);
        }
    }
    hash
}

pub(super) fn reflection_scene_signatures(
    objects: &[RenderObject],
    directional_lights: &[RenderDirectionalLight],
    point_lights: &[RenderPointLight],
    spot_lights: &[RenderSpotLight],
    environment: Option<&RenderEnvironment>,
    assets: Option<&RenderAssets>,
) -> (u64, [u64; 32]) {
    let mut global = 0xcbf29ce484222325_u64;
    if let Some(environment) = environment {
        reflection_hash_mix(&mut global, environment.primary.map_or(u64::MAX, |value| value.0));
        reflection_hash_mix(&mut global, environment.secondary.map_or(u64::MAX, |value| value.0));
        reflection_hash_mix(&mut global, environment.transition.to_bits() as u64);
        reflection_hash_mix(&mut global, environment.intensity.to_bits() as u64);
        reflection_hash_mix(&mut global, environment.rotation_radians.to_bits() as u64);
        reflection_hash_mix(&mut global, environment.draw_sky as u64);
        reflection_hash_mix(&mut global, environment.diffuse_ibl as u64);
        reflection_hash_mix(&mut global, environment.specular_ibl as u64);
        for handle in [environment.primary, environment.secondary].into_iter().flatten() {
            if let Some(cubemap) = assets.and_then(|assets| assets.cubemaps.get(&handle.0)) {
                reflection_hash_mix(&mut global, cubemap.revision);
                reflection_hash_mix(&mut global, cubemap.face_size as u64);
            }
        }
    }
    for light in directional_lights {
        reflection_hash_vec3(&mut global, light.direction);
        reflection_hash_vec3(&mut global, light.color);
        reflection_hash_mix(&mut global, light.intensity.to_bits() as u64);
        reflection_hash_mix(&mut global, light.shadow_mode as u64);
    }
    for light in point_lights {
        reflection_hash_vec3(&mut global, light.position);
        reflection_hash_vec3(&mut global, light.color);
        reflection_hash_mix(&mut global, light.intensity.to_bits() as u64);
        reflection_hash_mix(&mut global, light.range.unwrap_or(0.0).to_bits() as u64);
        reflection_hash_mix(&mut global, light.shadow_mode as u64);
    }
    for light in spot_lights {
        reflection_hash_vec3(&mut global, light.position);
        reflection_hash_vec3(&mut global, light.direction);
        reflection_hash_vec3(&mut global, light.color);
        reflection_hash_mix(&mut global, light.intensity.to_bits() as u64);
        reflection_hash_mix(&mut global, light.range.unwrap_or(0.0).to_bits() as u64);
        reflection_hash_mix(&mut global, light.inner_cone_angle.to_bits() as u64);
        reflection_hash_mix(&mut global, light.outer_cone_angle.to_bits() as u64);
        reflection_hash_mix(&mut global, light.shadow_mode as u64);
    }

    let mut layers = [0xcbf29ce484222325_u64; 32];
    for object in objects {
        let signature = reflection_object_signature(object, assets);
        for (bit, layer_signature) in layers.iter_mut().enumerate() {
            if object.render_layers & (1_u32 << bit) != 0 {
                reflection_hash_mix(layer_signature, signature);
            }
        }
    }
    (global, layers)
}
