//! Production environment-map import helpers.
//!
//! The runtime renderer only consumes canonical `CubemapAsset` data. This
//! module performs source decoding/conversion behind the `environment_import`
//! feature so games that ship fully cooked assets do not pay for image codecs.

use std::f32::consts::PI;
use std::path::Path;

use half::f16;

use super::CubemapAsset;

impl CubemapAsset {
    /// Decodes an HDR/EXR/LDR equirectangular image and converts it to a linear
    /// HDR cubemap. The `image` crate determines the source format.
    pub fn load_equirectangular(
        path: impl AsRef<Path>,
        face_size: u32,
    ) -> Result<Self, String> {
        let path = path.as_ref();
        let decoded = image::ImageReader::open(path)
            .map_err(|error| format!("failed to open environment image {}: {error}", path.display()))?
            .with_guessed_format()
            .map_err(|error| format!("failed to detect environment image format {}: {error}", path.display()))?
            .decode()
            .map_err(|error| format!("failed to decode environment image {}: {error}", path.display()))?
            .to_rgba32f();
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("environment")
            .to_string();
        Self::from_equirectangular_rgba32f(
            name,
            decoded.width(),
            decoded.height(),
            decoded.as_raw(),
            face_size,
        )
    }

    /// Converts a tightly-packed linear RGBA32F equirectangular image into the
    /// engine's canonical +X, -X, +Y, -Y, +Z, -Z cubemap orientation.
    pub fn from_equirectangular_rgba32f(
        name: impl Into<String>,
        width: u32,
        height: u32,
        pixels: &[f32],
        face_size: u32,
    ) -> Result<Self, String> {
        if width < 2 || height < 2 || face_size == 0 {
            return Err("equirectangular environments require non-zero dimensions and at least a 2x2 source".to_string());
        }
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            return Err(format!(
                "equirectangular RGBA32F source has {} values; expected {expected}",
                pixels.len()
            ));
        }
        let faces: [Vec<u16>; CubemapAsset::FACE_COUNT] = std::array::from_fn(|face| {
            let mut output = Vec::with_capacity(face_size as usize * face_size as usize * 4);
            for y in 0..face_size {
                for x in 0..face_size {
                    let u = 2.0 * (x as f32 + 0.5) / face_size as f32 - 1.0;
                    let v = 2.0 * (y as f32 + 0.5) / face_size as f32 - 1.0;
                    let direction = cubemap_face_direction(face, u, v);
                    let longitude = direction.z.atan2(direction.x);
                    let latitude = direction.y.clamp(-1.0, 1.0).asin();
                    let source_u = (0.5 + longitude / (2.0 * PI)).rem_euclid(1.0);
                    let source_v = (0.5 - latitude / PI).clamp(0.0, 1.0);
                    let sample = bilinear_equirectangular(pixels, width, height, source_u, source_v);
                    output.extend(sample.into_iter().map(|value| f16::from_f32(value.max(0.0)).to_bits()));
                }
            }
            output
        });
        CubemapAsset::from_faces_rgba16f(name, face_size, faces)
    }

    /// Loads a KTX2 cubemap. This compact reader intentionally supports the
    /// production formats the renderer can consume directly: uncompressed
    /// RGBA16F, RGBA32F, RGBA8 UNORM, and RGBA8 sRGB cube textures. Existing
    /// mip levels are preserved as an offline-prefiltered chain.
    pub fn load_ktx2(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)
            .map_err(|error| format!("failed to read KTX2 cubemap {}: {error}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("environment")
            .to_string();
        Self::from_ktx2_bytes(name, &bytes)
    }

    pub fn from_ktx2_bytes(name: impl Into<String>, bytes: &[u8]) -> Result<Self, String> {
        const IDENTIFIER: [u8; 12] = [
            0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
        ];
        if bytes.len() < 80 || bytes[..12] != IDENTIFIER {
            return Err("invalid KTX2 identifier or truncated header".to_string());
        }
        let vk_format = read_u32(bytes, 12)?;
        let width = read_u32(bytes, 20)?;
        let height = read_u32(bytes, 24)?;
        let depth = read_u32(bytes, 28)?;
        let layer_count = read_u32(bytes, 32)?;
        let face_count = read_u32(bytes, 36)?;
        let level_count = read_u32(bytes, 40)?.max(1);
        let supercompression = read_u32(bytes, 44)?;
        if width == 0 || width != height || depth != 0 || face_count != 6 {
            return Err("KTX2 environment must be a square two-dimensional cubemap with six faces".to_string());
        }
        if layer_count > 1 {
            return Err("cubemap arrays are not supported by CubemapAsset".to_string());
        }
        if supercompression != 0 {
            return Err("supercompressed KTX2 environments must be transcoded offline before import".to_string());
        }
        if bytes.len() < 80 + level_count as usize * 24 {
            return Err("truncated KTX2 level index".to_string());
        }

        let bytes_per_pixel = match vk_format {
            97 => 8,  // VK_FORMAT_R16G16B16A16_SFLOAT
            109 => 16, // VK_FORMAT_R32G32B32A32_SFLOAT
            37 | 43 => 4, // RGBA8 UNORM / SRGB
            _ => {
                return Err(format!(
                    "unsupported KTX2 Vulkan format {vk_format}; use RGBA16F, RGBA32F, RGBA8 UNORM, or RGBA8 sRGB"
                ));
            }
        };

        let mut mips = Vec::with_capacity(level_count as usize);
        let mut mip_size = width;
        for level in 0..level_count as usize {
            let index = 80 + level * 24;
            let offset = read_u64(bytes, index)? as usize;
            let length = read_u64(bytes, index + 8)? as usize;
            let expected = mip_size as usize
                * mip_size as usize
                * CubemapAsset::FACE_COUNT
                * bytes_per_pixel;
            let end = offset.checked_add(length).ok_or("KTX2 level range overflow")?;
            if end > bytes.len() || length < expected {
                return Err(format!(
                    "KTX2 mip {level} is truncated: {length} bytes, expected at least {expected}"
                ));
            }
            let source = &bytes[offset..offset + expected];
            let mut rgba16f = Vec::with_capacity(
                mip_size as usize * mip_size as usize * CubemapAsset::FACE_COUNT * 4,
            );
            match vk_format {
                97 => {
                    for channel in source.chunks_exact(2) {
                        rgba16f.push(u16::from_le_bytes([channel[0], channel[1]]));
                    }
                }
                109 => {
                    for channel in source.chunks_exact(4) {
                        let value = f32::from_le_bytes([channel[0], channel[1], channel[2], channel[3]]);
                        rgba16f.push(f16::from_f32(value.max(0.0)).to_bits());
                    }
                }
                37 | 43 => {
                    for pixel in source.chunks_exact(4) {
                        for (channel, value) in pixel.iter().copied().enumerate() {
                            let normalized = value as f32 / 255.0;
                            let linear = if vk_format == 43 && channel < 3 {
                                srgb_to_linear(normalized)
                            } else {
                                normalized
                            };
                            rgba16f.push(f16::from_f32(linear).to_bits());
                        }
                    }
                }
                _ => unreachable!(),
            }
            mips.push(rgba16f);
            mip_size = (mip_size / 2).max(1);
        }
        CubemapAsset::from_prefiltered_rgba16f_mips(name, width, mips)
    }
}

fn cubemap_face_direction(face: usize, u: f32, v: f32) -> glam::Vec3 {
    let direction = match face {
        0 => glam::Vec3::new(1.0, -v, -u),
        1 => glam::Vec3::new(-1.0, -v, u),
        2 => glam::Vec3::new(u, 1.0, v),
        3 => glam::Vec3::new(u, -1.0, -v),
        4 => glam::Vec3::new(u, -v, 1.0),
        _ => glam::Vec3::new(-u, -v, -1.0),
    };
    direction.normalize()
}

fn bilinear_equirectangular(
    pixels: &[f32],
    width: u32,
    height: u32,
    u: f32,
    v: f32,
) -> [f32; 4] {
    let x = u * width as f32 - 0.5;
    let y = v * height as f32 - 0.5;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x.floor();
    let ty = y - y.floor();
    let mut result = [0.0_f32; 4];
    for oy in 0..2 {
        for ox in 0..2 {
            let sx = (x0 + ox).rem_euclid(width as i32) as u32;
            let sy = (y0 + oy).clamp(0, height as i32 - 1) as u32;
            let weight_x = if ox == 0 { 1.0 - tx } else { tx };
            let weight_y = if oy == 0 { 1.0 - ty } else { ty };
            let weight = weight_x * weight_y;
            let index = ((sy * width + sx) * 4) as usize;
            for channel in 0..4 {
                result[channel] += pixels[index + channel] * weight;
            }
        }
    }
    result
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| "truncated KTX2 header".to_string())?;
    Ok(u32::from_le_bytes(slice.try_into().expect("four-byte slice")))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| "truncated KTX2 header".to_string())?;
    Ok(u64::from_le_bytes(slice.try_into().expect("eight-byte slice")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_constant_equirectangular_without_changing_radiance() {
        let source = vec![2.0_f32, 1.0, 0.5, 1.0].repeat(8);
        let cubemap = CubemapAsset::from_equirectangular_rgba32f("constant", 4, 2, &source, 4)
            .expect("valid conversion");
        let first = cubemap.face_rgba16f(0).unwrap();
        assert!((f16::from_bits(first[0]).to_f32() - 2.0).abs() < 0.01);
        assert!((f16::from_bits(first[1]).to_f32() - 1.0).abs() < 0.01);
    }

    #[test]
    fn rejects_supercompressed_ktx2() {
        let mut bytes = vec![0_u8; 104];
        bytes[..12].copy_from_slice(&[
            0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
        ]);
        bytes[12..16].copy_from_slice(&97_u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&1_u32.to_le_bytes());
        bytes[24..28].copy_from_slice(&1_u32.to_le_bytes());
        bytes[36..40].copy_from_slice(&6_u32.to_le_bytes());
        bytes[40..44].copy_from_slice(&1_u32.to_le_bytes());
        bytes[44..48].copy_from_slice(&1_u32.to_le_bytes());
        assert!(CubemapAsset::from_ktx2_bytes("bad", &bytes).is_err());
    }
}
