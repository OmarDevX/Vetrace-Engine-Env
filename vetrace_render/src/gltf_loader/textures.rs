use super::*;

pub(crate) fn import_textures(
    engine: &mut Engine,
    document: &gltf::Document,
    images: &[gltf::image::Data],
) -> Vec<Option<TextureHandle>> {
    let mut handles = vec![None; document.images().count()];
    for image in document.images() {
        let index = image.index();
        let Some(data) = images.get(index) else { continue; };
        let Some(texture) = texture_asset_from_gltf_image(image.name(), index, data) else { continue; };
        let handle = {
            let assets = render_assets_mut(engine);
            assets.insert_texture(texture)
        };
        if let Some(slot) = handles.get_mut(index) {
            *slot = Some(handle);
        }
    }
    handles
}

fn texture_asset_from_gltf_image(name: Option<&str>, index: usize, data: &gltf::image::Data) -> Option<TextureAsset> {
    if data.width == 0 || data.height == 0 { return None; }
    let rgba8 = rgba8_from_gltf_image(data)?;
    Some(TextureAsset {
        name: name.map(ToOwned::to_owned).unwrap_or_else(|| format!("gltf_image_{index}")),
        width: data.width,
        height: data.height,
        rgba8,
        revision: 0,
    })
}

fn rgba8_from_gltf_image(data: &gltf::image::Data) -> Option<Vec<u8>> {
    use gltf::image::Format;
    let pixel_count = data.width.checked_mul(data.height)? as usize;
    match data.format {
        Format::R8 => {
            if data.pixels.len() < pixel_count { return None; }
            let mut out = Vec::with_capacity(pixel_count * 4);
            for r in data.pixels.iter().copied().take(pixel_count) {
                out.extend_from_slice(&[r, r, r, 255]);
            }
            Some(out)
        }
        Format::R8G8 => {
            if data.pixels.len() < pixel_count * 2 { return None; }
            let mut out = Vec::with_capacity(pixel_count * 4);
            for px in data.pixels.chunks_exact(2).take(pixel_count) {
                out.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
            Some(out)
        }
        Format::R8G8B8 => {
            if data.pixels.len() < pixel_count * 3 { return None; }
            let mut out = Vec::with_capacity(pixel_count * 4);
            for px in data.pixels.chunks_exact(3).take(pixel_count) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            Some(out)
        }
        Format::R8G8B8A8 => {
            if data.pixels.len() < pixel_count * 4 { return None; }
            Some(data.pixels[..pixel_count * 4].to_vec())
        }
        // Keep unsupported HDR/16-bit formats non-fatal. The material still renders
        // through its base-color factor and the renderer binds the white fallback.
        _ => None,
    }
}
