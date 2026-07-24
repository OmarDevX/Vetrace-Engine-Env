use super::*;
use std::hash::{Hash, Hasher};

pub(super) fn environment_assets_signature(
    handles: &[crate::components::CubemapHandle],
    assets: Option<&RenderAssets>,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    handles.len().hash(&mut hasher);
    for handle in handles {
        handle.0.hash(&mut hasher);
        if let Some(asset) = assets.and_then(|assets| assets.cubemaps.get(&handle.0)) {
            asset.face_size.hash(&mut hasher);
            asset.rgba8.len().hash(&mut hasher);
            asset.rgba16f.len().hash(&mut hasher);
            asset.prefiltered_rgba16f_mips.len().hash(&mut hasher);
            for mip in &asset.prefiltered_rgba16f_mips {
                mip.len().hash(&mut hasher);
            }
            asset.revision.hash(&mut hasher);
            asset.is_valid().hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(super) fn upload_cubemap_asset(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    slot: u32,
    asset: &crate::resources::CubemapAsset,
) {
    if asset.is_prefiltered() {
        upload_prefiltered_cubemap_asset(queue, texture, slot, asset);
        return;
    }
    if asset.is_hdr() {
        for face in 0..crate::resources::CubemapAsset::FACE_COUNT {
            let Some(source) = asset.face_rgba16f(face) else { continue; };
            let mut mip = resample_linear_rgba16f_face(
                source,
                asset.face_size,
                ENVIRONMENT_CUBEMAP_FACE_SIZE,
            );
            let mut size = ENVIRONMENT_CUBEMAP_FACE_SIZE;
            for mip_level in 0..ENVIRONMENT_CUBEMAP_MIP_COUNT {
                write_rgba16f_bits_texture_layer(
                    queue,
                    texture,
                    mip_level,
                    slot * 6 + face as u32,
                    size,
                    &mip,
                );
                if size > 1 {
                    mip = downsample_linear_rgba16f_face(&mip, size);
                    size = (size / 2).max(1);
                }
            }
        }
        return;
    }
    for face in 0..crate::resources::CubemapAsset::FACE_COUNT {
        let Some(source) = asset.face_rgba8(face) else { continue; };
        let mut mip = resample_srgb_face(
            source,
            asset.face_size,
            ENVIRONMENT_CUBEMAP_FACE_SIZE,
        );
        let mut size = ENVIRONMENT_CUBEMAP_FACE_SIZE;
        for mip_level in 0..ENVIRONMENT_CUBEMAP_MIP_COUNT {
            write_rgba16f_texture_layer(
                queue,
                texture,
                mip_level,
                slot * 6 + face as u32,
                size,
                &mip,
            );
            if size > 1 {
                mip = downsample_srgb_face_linear(&mip, size);
                size = (size / 2).max(1);
            }
        }
    }
}

pub(super) fn upload_prefiltered_cubemap_asset(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    slot: u32,
    asset: &crate::resources::CubemapAsset,
) {
    let source_shift = if asset.face_size > ENVIRONMENT_CUBEMAP_FACE_SIZE
        && asset.face_size.is_power_of_two()
        && ENVIRONMENT_CUBEMAP_FACE_SIZE.is_power_of_two()
    {
        (asset.face_size / ENVIRONMENT_CUBEMAP_FACE_SIZE).ilog2() as usize
    } else {
        0
    };
    for target_mip in 0..ENVIRONMENT_CUBEMAP_MIP_COUNT {
        let source_mip = (target_mip as usize + source_shift)
            .min(asset.prefiltered_rgba16f_mips.len().saturating_sub(1));
        let source_size = (asset.face_size >> source_mip).max(1);
        let target_size = (ENVIRONMENT_CUBEMAP_FACE_SIZE >> target_mip).max(1);
        let Some(packed) = asset.prefiltered_mip_rgba16f(source_mip) else { continue; };
        let source_face_stride = source_size as usize * source_size as usize * 4;
        for face in 0..crate::resources::CubemapAsset::FACE_COUNT {
            let start = face * source_face_stride;
            let end = start + source_face_stride;
            if end > packed.len() { continue; }
            let pixels = resample_linear_rgba16f_face(
                &packed[start..end],
                source_size,
                target_size,
            );
            write_rgba16f_bits_texture_layer(
                queue,
                texture,
                target_mip,
                slot * 6 + face as u32,
                target_size,
                &pixels,
            );
        }
    }
}

pub(super) fn write_rgba16f_texture_layer(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    array_layer: u32,
    size: u32,
    pixels: &[u8],
) {
    let unpadded_row = size as usize * 8;
    let padded_row = (unpadded_row + 255) & !255;
    let mut padded = vec![0_u8; padded_row * size as usize];
    for row in 0..size as usize {
        for column in 0..size as usize {
            let source = (row * size as usize + column) * 4;
            let destination = row * padded_row + column * 8;
            let channels = [
                f16::from_f32(srgb8_to_linear(pixels[source])),
                f16::from_f32(srgb8_to_linear(pixels[source + 1])),
                f16::from_f32(srgb8_to_linear(pixels[source + 2])),
                f16::from_f32(pixels[source + 3] as f32 / 255.0),
            ];
            for (channel, value) in channels.into_iter().enumerate() {
                let bytes = value.to_bits().to_le_bytes();
                let offset = destination + channel * 2;
                padded[offset..offset + 2].copy_from_slice(&bytes);
            }
        }
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level,
            origin: wgpu::Origin3d { x: 0, y: 0, z: array_layer },
            aspect: wgpu::TextureAspect::All,
        },
        &padded,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(padded_row as u32),
            rows_per_image: Some(size),
        },
        wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
    );
}

pub(super) fn resample_srgb_face(source: &[u8], source_size: u32, target_size: u32) -> Vec<u8> {
    if source_size == target_size {
        return source.to_vec();
    }
    let mut output = vec![0_u8; target_size as usize * target_size as usize * 4];
    let source_size_f = source_size as f32;
    for y in 0..target_size {
        for x in 0..target_size {
            let sx = (((x as f32 + 0.5) / target_size as f32) * source_size_f)
                .floor()
                .clamp(0.0, source_size_f - 1.0) as u32;
            let sy = (((y as f32 + 0.5) / target_size as f32) * source_size_f)
                .floor()
                .clamp(0.0, source_size_f - 1.0) as u32;
            let src = ((sy * source_size + sx) * 4) as usize;
            let dst = ((y * target_size + x) * 4) as usize;
            output[dst..dst + 4].copy_from_slice(&source[src..src + 4]);
        }
    }
    output
}

pub(super) fn downsample_srgb_face_linear(source: &[u8], source_size: u32) -> Vec<u8> {
    let target_size = (source_size / 2).max(1);
    let mut output = vec![0_u8; target_size as usize * target_size as usize * 4];
    for y in 0..target_size {
        for x in 0..target_size {
            let mut linear = [0.0_f32; 3];
            let mut alpha = 0.0_f32;
            let mut samples = 0.0_f32;
            for oy in 0..2 {
                for ox in 0..2 {
                    let sx = (x * 2 + ox).min(source_size - 1);
                    let sy = (y * 2 + oy).min(source_size - 1);
                    let index = ((sy * source_size + sx) * 4) as usize;
                    for channel in 0..3 {
                        linear[channel] += srgb8_to_linear(source[index + channel]);
                    }
                    alpha += source[index + 3] as f32 / 255.0;
                    samples += 1.0;
                }
            }
            let dst = ((y * target_size + x) * 4) as usize;
            for channel in 0..3 {
                output[dst + channel] = linear_to_srgb8(linear[channel] / samples);
            }
            output[dst + 3] = ((alpha / samples) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    output
}

pub(super) fn srgb8_to_linear(value: u8) -> f32 {
    let value = value as f32 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

pub(super) fn linear_to_srgb8(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let srgb = if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round().clamp(0.0, 255.0) as u8
}

pub(super) fn write_rgba16f_bits_texture_layer(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip_level: u32,
    array_layer: u32,
    size: u32,
    pixels: &[u16],
) {
    let expected = size as usize * size as usize * 4;
    if pixels.len() != expected {
        return;
    }
    let unpadded_row = size as usize * 8;
    let padded_row = (unpadded_row + 255) & !255;
    let mut padded = vec![0_u8; padded_row * size as usize];
    for row in 0..size as usize {
        let source_start = row * size as usize * 4;
        let destination_start = row * padded_row;
        let source_bytes = bytemuck::cast_slice(&pixels[source_start..source_start + size as usize * 4]);
        padded[destination_start..destination_start + unpadded_row].copy_from_slice(source_bytes);
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level,
            origin: wgpu::Origin3d { x: 0, y: 0, z: array_layer },
            aspect: wgpu::TextureAspect::All,
        },
        &padded,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(padded_row as u32),
            rows_per_image: Some(size),
        },
        wgpu::Extent3d { width: size, height: size, depth_or_array_layers: 1 },
    );
}

pub(super) fn resample_linear_rgba16f_face(source: &[u16], source_size: u32, target_size: u32) -> Vec<u16> {
    if source_size == target_size {
        return source.to_vec();
    }
    let mut output = vec![0_u16; target_size as usize * target_size as usize * 4];
    let source_max = source_size.saturating_sub(1) as f32;
    for y in 0..target_size {
        for x in 0..target_size {
            let source_x = ((x as f32 + 0.5) * source_size as f32 / target_size as f32 - 0.5)
                .clamp(0.0, source_max);
            let source_y = ((y as f32 + 0.5) * source_size as f32 / target_size as f32 - 0.5)
                .clamp(0.0, source_max);
            let x0 = source_x.floor() as u32;
            let y0 = source_y.floor() as u32;
            let x1 = (x0 + 1).min(source_size - 1);
            let y1 = (y0 + 1).min(source_size - 1);
            let tx = source_x - x0 as f32;
            let ty = source_y - y0 as f32;
            let destination = ((y * target_size + x) * 4) as usize;
            for channel in 0..4 {
                let sample = |sx: u32, sy: u32| {
                    let index = ((sy * source_size + sx) * 4) as usize + channel;
                    f16::from_bits(source[index]).to_f32()
                };
                let top = sample(x0, y0) * (1.0 - tx) + sample(x1, y0) * tx;
                let bottom = sample(x0, y1) * (1.0 - tx) + sample(x1, y1) * tx;
                output[destination + channel] = f16::from_f32(top * (1.0 - ty) + bottom * ty).to_bits();
            }
        }
    }
    output
}

pub(super) fn downsample_linear_rgba16f_face(source: &[u16], source_size: u32) -> Vec<u16> {
    let target_size = (source_size / 2).max(1);
    let mut output = vec![0_u16; target_size as usize * target_size as usize * 4];
    for y in 0..target_size {
        for x in 0..target_size {
            let destination = ((y * target_size + x) * 4) as usize;
            for channel in 0..4 {
                let mut sum = 0.0_f32;
                let mut count = 0.0_f32;
                for oy in 0..2 {
                    for ox in 0..2 {
                        let sx = (x * 2 + ox).min(source_size - 1);
                        let sy = (y * 2 + oy).min(source_size - 1);
                        let index = ((sy * source_size + sx) * 4) as usize + channel;
                        sum += f16::from_bits(source[index]).to_f32();
                        count += 1.0;
                    }
                }
                output[destination + channel] = f16::from_f32(sum / count).to_bits();
            }
        }
    }
    output
}
