use super::*;

// Static receiver lightmap rasterization.

#[allow(clippy::too_many_arguments)]
pub(super) fn rasterize_object_lightmap(
    key: u64,
    tile: Tile,
    atlas_width: u32,
    combined_atlas: &mut [Vec3],
    indirect_atlas: &mut [Vec3],
    coverage: &mut [bool],
    triangles: &[BakeTriangle],
    probes: &BakedProbeGrid,
    directional: &[RenderDirectionalLight],
    points: &[RenderPointLight],
    spots: &[RenderSpotLight],
    areas: &[BakeRectAreaLight],
    config: &BakedLightingBakeConfig,
) {
    for (triangle_index, triangle) in triangles.iter().enumerate().filter(|(_, triangle)| triangle.object_key == key) {
        let Some(uvs) = triangle.lightmap_uvs else { continue; };
        let points_px = uvs.map(|uv| Vec2::new(
            tile.x as f32 + uv.x.clamp(0.0, 1.0) * tile.resolution.saturating_sub(1) as f32,
            tile.y as f32 + uv.y.clamp(0.0, 1.0) * tile.resolution.saturating_sub(1) as f32,
        ));
        let min = points_px[0].min(points_px[1]).min(points_px[2]).floor();
        let max = points_px[0].max(points_px[1]).max(points_px[2]).ceil();
        let min_x = min.x.max(tile.x as f32) as u32;
        let min_y = min.y.max(tile.y as f32) as u32;
        let max_x = max.x.min((tile.x + tile.resolution - 1) as f32) as u32;
        let max_y = max.y.min((tile.y + tile.resolution - 1) as f32) as u32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let sample = Vec2::new(x as f32, y as f32);
                let Some(bary) = barycentric_2d(sample, points_px) else { continue; };
                if bary.min_element() < -0.001 { continue; }
                let position = triangle.positions[0] * bary.x + triangle.positions[1] * bary.y + triangle.positions[2] * bary.z;
                let normal = (triangle.normals[0] * bary.x + triangle.normals[1] * bary.y + triangle.normals[2] * bary.z).normalize_or_zero();
                let direct = direct_irradiance(
                    position,
                    normal,
                    Some(triangle_index),
                    triangles,
                    directional,
                    points,
                    spots,
                    areas,
                    config.surface_bias,
                );
                let indirect = probes.sample(position).irradiance_for_normal(normal) * config.indirect_intensity.max(0.0);
                let max_radiance = Vec3::splat(config.max_baked_radiance.min(65_504.0));
                let pixel_index = (y * atlas_width + x) as usize;
                combined_atlas[pixel_index] = (direct + indirect).clamp(Vec3::ZERO, max_radiance);
                indirect_atlas[pixel_index] = indirect.clamp(Vec3::ZERO, max_radiance);
                coverage[pixel_index] = true;
            }
        }
    }
}
