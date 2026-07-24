use super::*;

pub(crate) fn import_materials(engine: &mut Engine, document: &gltf::Document, texture_handles: &[Option<TextureHandle>]) -> Vec<MaterialHandle> {
    let mut handles = Vec::new();
    for material in document.materials() {
        let handle = {
            let assets = render_assets_mut(engine);
            assets.insert_material(material_from_gltf(&material, texture_handles))
        };
        handles.push(handle);
    }
    handles
}

fn material_from_gltf(material: &gltf::Material, texture_handles: &[Option<TextureHandle>]) -> Material {
    let pbr = material.pbr_metallic_roughness();
    let base = pbr.base_color_factor();
    let emissive = material.emissive_factor();
    let alpha_mode = match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask,
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    };
    let alpha = match alpha_mode {
        AlphaMode::Opaque => 1.0,
        AlphaMode::Mask | AlphaMode::Blend => base[3].clamp(0.0, 1.0),
    };
    let base_color_texture = pbr
        .base_color_texture()
        .map(|info| info.texture().source().index())
        .and_then(|index| texture_handles.get(index).copied().flatten());
    let metallic_roughness_texture = pbr
        .metallic_roughness_texture()
        .map(|info| info.texture().source().index())
        .and_then(|index| texture_handles.get(index).copied().flatten());
    let normal_info = material.normal_texture();
    let normal_texture = normal_info
        .as_ref()
        .map(|info| info.texture().source().index())
        .and_then(|index| texture_handles.get(index).copied().flatten());
    let occlusion_info = material.occlusion_texture();
    let occlusion_texture = occlusion_info
        .as_ref()
        .map(|info| info.texture().source().index())
        .and_then(|index| texture_handles.get(index).copied().flatten());
    let emissive_texture = material
        .emissive_texture()
        .map(|info| info.texture().source().index())
        .and_then(|index| texture_handles.get(index).copied().flatten());

    Material {
        base_color: Vec3::new(base[0], base[1], base[2]),
        base_color_texture,
        base_color_texture_path: None,
        uv_scale: Vec2::ONE,
        normal_texture,
        metallic_roughness_texture,
        occlusion_texture,
        emissive_texture,
        emissive: Vec3::new(emissive[0], emissive[1], emissive[2]),
        roughness: pbr.roughness_factor().clamp(0.04, 1.0),
        metallic: pbr.metallic_factor().clamp(0.0, 1.0),
        alpha,
        alpha_mode,
        alpha_cutoff: material.alpha_cutoff().unwrap_or(0.5).clamp(0.0, 1.0),
        double_sided: material.double_sided(),
        normal_scale: normal_info.map(|info| info.scale()).unwrap_or(1.0).max(0.0),
        occlusion_strength: occlusion_info.map(|info| info.strength()).unwrap_or(1.0).clamp(0.0, 1.0),
        is_glass: false,
        specular_f0: Vec3::splat(0.04),
        ior: 1.5,
    }
}
