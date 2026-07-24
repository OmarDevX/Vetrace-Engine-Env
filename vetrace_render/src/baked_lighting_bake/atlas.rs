use super::*;

// Lightmap sizing, packing, filtering, dilation, and RGBA16F packing.

pub(super) fn receiver_lightmap_resolution(
    triangles: &[BakeTriangle],
    config: &BakedLightingBakeConfig,
    resolution_scale: f32,
) -> u32 {
    let mut required = config.lightmap_resolution.max(4) as f32;
    let density = config.lightmap_texels_per_unit.max(0.0);
    if density > 0.0 {
        for triangle in triangles {
            let Some(uvs) = triangle.lightmap_uvs else { continue; };
            for (a, b) in [(0_usize, 1_usize), (1, 2), (2, 0)] {
                let uv_length = uvs[a].distance(uvs[b]);
                if uv_length <= 1.0e-6 { continue; }
                let world_length = triangle.positions[a].distance(triangle.positions[b]);
                if world_length.is_finite() {
                    required = required.max(world_length * density / uv_length);
                }
            }
        }
    }
    (required * resolution_scale.max(0.125))
        .ceil()
        .clamp(4.0, 512.0) as u32
}

pub(super) fn pack_tiles(resolutions: &HashMap<u64, u32>, padding: u32) -> Result<(HashMap<u64, Tile>, u32, u32), Box<dyn Error>> {
    if resolutions.is_empty() { return Err("no receiver has usable lightmap UVs".into()); }
    let mut entries = resolutions.iter().map(|(key, resolution)| (*key, *resolution)).collect::<Vec<_>>();
    entries.sort_by_key(|(key, _)| *key);
    let max_res = entries.iter().map(|(_, resolution)| *resolution).max().unwrap_or(4);
    let cell = max_res + padding.saturating_mul(2);
    let columns = (entries.len() as f32).sqrt().ceil() as u32;
    let rows = (entries.len() as u32).div_ceil(columns.max(1));
    let packed_width = columns
        .checked_mul(cell)
        .ok_or("baked-lightmap atlas width overflow")?;
    let packed_height = rows
        .checked_mul(cell)
        .ok_or("baked-lightmap atlas height overflow")?;
    let width = next_power_of_two(packed_width.max(4));
    let height = next_power_of_two(packed_height.max(4));
    if width > 8192 || height > 4096 {
        return Err("baked-lightmap atlas layers would exceed 8192x4096 (8192x8192 physical)".into());
    }
    let mut out = HashMap::new();
    for (index, (key, resolution)) in entries.into_iter().enumerate() {
        let col = index as u32 % columns;
        let row = index as u32 / columns;
        out.insert(key, Tile { x: col * cell + padding, y: row * cell + padding, resolution });
    }
    Ok((out, width, height))
}

pub(super) fn filter_lightmap_tile(
    tile: Tile,
    atlas_width: u32,
    atlas: &mut [Vec3],
    coverage: &[bool],
    radius: u32,
) {
    let radius = radius.min(8) as i32;
    if radius <= 0 {
        return;
    }
    let source = atlas.to_vec();
    for y in tile.y..tile.y + tile.resolution {
        for x in tile.x..tile.x + tile.resolution {
            let pixel_index = (y * atlas_width + x) as usize;
            if !coverage[pixel_index] {
                continue;
            }
            let mut sum = Vec3::ZERO;
            let mut total_weight = 0.0_f32;
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < tile.x as i32
                        || ny < tile.y as i32
                        || nx >= (tile.x + tile.resolution) as i32
                        || ny >= (tile.y + tile.resolution) as i32
                    {
                        continue;
                    }
                    let neighbor = (ny as u32 * atlas_width + nx as u32) as usize;
                    if !coverage[neighbor] {
                        continue;
                    }
                    let wx = (radius + 1 - dx.abs()) as f32;
                    let wy = (radius + 1 - dy.abs()) as f32;
                    let weight = wx * wy;
                    sum += source[neighbor] * weight;
                    total_weight += weight;
                }
            }
            if total_weight > 0.0 {
                atlas[pixel_index] = sum / total_weight;
            }
        }
    }
}

pub(super) fn dilate_lightmap_tile(
    tile: Tile,
    atlas_width: u32,
    atlas: &mut [Vec3],
    coverage: &mut [bool],
    passes: u32,
) {
    for _ in 0..passes {
        let mut fills = Vec::new();
        for y in tile.y..tile.y + tile.resolution {
            for x in tile.x..tile.x + tile.resolution {
                let pixel_index = (y * atlas_width + x) as usize;
                if coverage[pixel_index] {
                    continue;
                }
                let mut sum = Vec3::ZERO;
                let mut count = 0_u32;
                for dy in -1_i32..=1 {
                    for dx in -1_i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < tile.x as i32
                            || ny < tile.y as i32
                            || nx >= (tile.x + tile.resolution) as i32
                            || ny >= (tile.y + tile.resolution) as i32
                        {
                            continue;
                        }
                        let neighbor = (ny as u32 * atlas_width + nx as u32) as usize;
                        if !coverage[neighbor] {
                            continue;
                        }
                        sum += atlas[neighbor];
                        count += 1;
                    }
                }
                if count > 0 {
                    fills.push((pixel_index, sum / count as f32));
                }
            }
        }
        if fills.is_empty() {
            break;
        }
        for (pixel_index, color) in fills {
            atlas[pixel_index] = color;
            coverage[pixel_index] = true;
        }
    }
}

pub(super) fn pack_rgba16f_atlas(combined: Vec<Vec3>, indirect: Vec<Vec3>) -> Vec<u16> {
    let mut out = Vec::with_capacity((combined.len() + indirect.len()) * 4);
    for color in combined.into_iter().chain(indirect) {
        let color = color.max(Vec3::ZERO).min(Vec3::splat(65_504.0));
        out.push(f32_to_f16_bits(color.x));
        out.push(f32_to_f16_bits(color.y));
        out.push(f32_to_f16_bits(color.z));
        out.push(f32_to_f16_bits(1.0));
    }
    out
}

/// Converts finite non-negative f32 lighting values to IEEE-754 binary16 using
/// round-to-nearest-even. Values beyond the finite half range are saturated.
pub(super) fn f32_to_f16_bits(value: f32) -> u16 {
    let value = value.clamp(0.0, 65_504.0);
    let bits = value.to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32;
    let mantissa = bits & 0x7f_ffff;

    if exponent == 0xff {
        return 0x7bff;
    }

    let half_exponent = exponent - 127 + 15;
    if half_exponent >= 31 {
        return 0x7bff;
    }
    if half_exponent <= 0 {
        if half_exponent < -10 {
            return 0;
        }
        let significand = mantissa | 0x80_0000;
        let shift = (14 - half_exponent) as u32;
        let mut half_mantissa = (significand >> shift) as u16;
        let remainder_mask = (1_u32 << shift) - 1;
        let remainder = significand & remainder_mask;
        let halfway = 1_u32 << (shift - 1);
        if remainder > halfway || (remainder == halfway && (half_mantissa & 1) != 0) {
            half_mantissa = half_mantissa.saturating_add(1);
        }
        return half_mantissa;
    }

    let mut half = ((half_exponent as u16) << 10) | ((mantissa >> 13) as u16);
    let remainder = mantissa & 0x1fff;
    if remainder > 0x1000 || (remainder == 0x1000 && (half & 1) != 0) {
        half = half.saturating_add(1);
    }
    half.min(0x7bff)
}

#[cfg(test)]
pub(super) fn f16_bits_to_f32(bits: u16) -> f32 {
    let sign = ((bits & 0x8000) as u32) << 16;
    let exponent = ((bits >> 10) & 0x1f) as u32;
    let mantissa = (bits & 0x03ff) as u32;
    let out = match exponent {
        0 => {
            if mantissa == 0 {
                sign
            } else {
                let leading = mantissa.leading_zeros() - 22;
                let normalized_mantissa = (mantissa << (leading + 1)) & 0x03ff;
                let exponent32 = 127 - 15 - leading;
                sign | (exponent32 << 23) | (normalized_mantissa << 13)
            }
        }
        0x1f => sign | 0x7f80_0000 | (mantissa << 13),
        _ => sign | ((exponent + 112) << 23) | (mantissa << 13),
    };
    f32::from_bits(out)
}

pub(super) fn next_power_of_two(value: u32) -> u32 {
    value.checked_next_power_of_two().unwrap_or(value)
}
