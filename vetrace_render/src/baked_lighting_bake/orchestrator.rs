use super::*;

// Top-level CPU bake orchestration.

pub(crate) fn bake_baked_lighting(
    engine: &Engine,
    config: &BakedLightingBakeConfig,
) -> Result<(BakedLightingFile, BakedLightingBakeReport), Box<dyn Error>> {
    validate_bake_config(config)?;

    let frame = build_render_frame(engine);
    let assets = engine.get_resource::<RenderAssets>();
    let area_lights = extract_baked_rect_area_lights(engine)?;
    let area_light_entities = area_lights
        .iter()
        .map(|light| light.entity)
        .collect::<HashSet<_>>();
    let receivers: HashMap<_, _> = engine
        .actors_with::<BakedLightmapReceiver>()
        .into_iter()
        .filter(|(_, receiver)| receiver.enabled)
        .map(|(actor, receiver)| (actor.entity(), *receiver))
        .collect();

    let mut triangles = Vec::new();
    let mut receiver_resolutions = HashMap::new();
    let mut receiver_keys = HashSet::new();
    let mut skipped = 0usize;
    for object in &frame.objects {
        let Some(receiver) = receivers.get(&object.entity).copied() else { continue; };
        if !receiver.resolution_scale.is_finite() || receiver.resolution_scale <= 0.0 {
            return Err(format!(
                "baked-lightmap receiver `{}` has an invalid resolution_scale",
                object.name.as_deref().unwrap_or("<unnamed>"),
            )
            .into());
        }
        let key = baked_object_key(object, assets);
        let before = triangles.len();
        append_object_triangles(object, assets, key, &mut triangles);
        if area_light_entities.contains(&object.entity) {
            // The visible material may be strongly emissive for display, but the
            // rectangular emitter is sampled explicitly. Suppressing triangle
            // emission avoids counting the same source twice in the probe solve.
            for triangle in &mut triangles[before..] {
                triangle.emissive = Vec3::ZERO;
            }
        }
        if triangles[before..].iter().any(|triangle| triangle.lightmap_uvs.is_some()) {
            if receiver_keys.contains(&key) {
                return Err(format!(
                    "duplicate baked-lighting object key for `{}`; give duplicate static objects distinct names or transforms",
                    object.name.as_deref().unwrap_or("<unnamed>"),
                ).into());
            }
            let resolution = receiver_lightmap_resolution(
                &triangles[before..],
                config,
                receiver.resolution_scale,
            );
            receiver_resolutions.insert(key, resolution);
            receiver_keys.insert(key);
        } else {
            skipped += 1;
        }
    }
    if triangles.is_empty() {
        return Err("no enabled baked-lightmap receiver geometry was found".into());
    }
    if triangles.iter().any(|triangle| {
        triangle.positions.iter().any(|value| !value.is_finite())
            || triangle.normals.iter().any(|value| !value.is_finite())
            || !triangle.albedo.is_finite()
            || !triangle.emissive.is_finite()
    }) {
        return Err("bake geometry contains non-finite transform or material data".into());
    }

    let (bounds_min, bounds_max) =
        triangle_bounds(&triangles).ok_or("bake geometry has invalid bounds")?;
    let indirect_probes = bake_probe_grid(
        &triangles,
        &frame.directional_lights,
        &frame.point_lights,
        &frame.spot_lights,
        &area_lights,
        bounds_min,
        bounds_max,
        config,
    );
    let probes = add_direct_lighting_to_probe_grid(
        &indirect_probes,
        &triangles,
        &frame.directional_lights,
        &frame.point_lights,
        &frame.spot_lights,
        &area_lights,
        config,
    );

    let (tiles, atlas_width, logical_atlas_height) = pack_tiles(&receiver_resolutions, config.atlas_padding)?;
    let layer_pixels = atlas_width as usize * logical_atlas_height as usize;
    // Keep the entire bake/filter/dilation path in float precision. Conversion
    // to binary16 happens only once after all CPU processing is complete.
    let mut combined_atlas = vec![Vec3::ZERO; layer_pixels];
    let mut indirect_atlas = vec![Vec3::ZERO; layer_pixels];
    let mut coverage = vec![false; layer_pixels];
    let mut lightmaps = Vec::new();
    let physical_atlas_height = logical_atlas_height
        .checked_mul(2)
        .ok_or("baked-lightmap physical atlas height overflow")?;
    for (&key, tile) in &tiles {
        rasterize_object_lightmap(
            key,
            *tile,
            atlas_width,
            &mut combined_atlas,
            &mut indirect_atlas,
            &mut coverage,
            &triangles,
            &indirect_probes,
            &frame.directional_lights,
            &frame.point_lights,
            &frame.spot_lights,
            &area_lights,
            config,
        );
        filter_lightmap_tile(
            *tile,
            atlas_width,
            &mut combined_atlas,
            &coverage,
            config.lightmap_filter_radius,
        );
        filter_lightmap_tile(
            *tile,
            atlas_width,
            &mut indirect_atlas,
            &coverage,
            config.lightmap_filter_radius,
        );
        let mut combined_coverage = coverage.clone();
        let mut indirect_coverage = coverage.clone();
        dilate_lightmap_tile(*tile, atlas_width, &mut combined_atlas, &mut combined_coverage, config.atlas_padding.max(4));
        dilate_lightmap_tile(*tile, atlas_width, &mut indirect_atlas, &mut indirect_coverage, config.atlas_padding.max(4));
        let inset = 0.5_f32;
        let scale = Vec2::splat((tile.resolution.saturating_sub(1)) as f32)
            / Vec2::new(atlas_width as f32, physical_atlas_height as f32);
        let offset = Vec2::new(tile.x as f32 + inset, tile.y as f32 + inset)
            / Vec2::new(atlas_width as f32, physical_atlas_height as f32);
        let receiver = frame.objects.iter().find(|object| baked_object_key(object, assets) == key)
            .and_then(|object| receivers.get(&object.entity)).copied().unwrap_or_default();
        lightmaps.push((key, BakedLightmapRegion {
            uv_scale_offset: Vec4::new(scale.x, scale.y, offset.x, offset.y),
            intensity: config.lightmap_intensity.max(0.0),
            static_lighting_only: receiver.static_lighting_only,
            preserve_local_lights: receiver.preserve_local_lights,
        }));
    }

    // The atlas keeps the one-sample runtime path: combined direct+indirect is
    // stored in the top half, while an indirect-only copy is stored in the
    // bottom half for high-quality hybrid realtime shadows.
    let atlas_rgba16f = pack_rgba16f_atlas(combined_atlas, indirect_atlas);

    let file = BakedLightingFile {
        version: BAKED_LIGHTING_FILE_VERSION,
        source_name: config.source_name.clone(),
        atlas_width,
        atlas_height: physical_atlas_height,
        atlas_rgba16f,
        lightmaps,
        probes,
        indirect_probes,
    };
    let report = BakedLightingBakeReport {
        receiver_count: receivers.len(),
        baked_receiver_count: receiver_keys.len(),
        skipped_receiver_count: skipped,
        triangle_count: triangles.len(),
        atlas_width,
        atlas_height: physical_atlas_height,
        min_lightmap_resolution: receiver_resolutions.values().copied().min().unwrap_or(0),
        max_lightmap_resolution: receiver_resolutions.values().copied().max().unwrap_or(0),
        probe_count: file.probes.samples.len(),
        output_bytes: 0,
    };
    Ok((file, report))
}
