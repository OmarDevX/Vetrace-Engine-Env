use super::*;

// Bake input extraction and validation.

pub(super) fn extract_baked_rect_area_lights(
    engine: &Engine,
) -> Result<Vec<BakeRectAreaLight>, Box<dyn Error>> {
    let mut out = Vec::new();
    for (actor, light) in engine.actors_with::<BakedRectAreaLight>() {
        if !light.enabled {
            continue;
        }
        if !light.color.is_finite()
            || light.color.min_element() < 0.0
            || !light.intensity.is_finite()
            || light.intensity < 0.0
            || !light.width.is_finite()
            || light.width <= 0.0
            || !light.height.is_finite()
            || light.height <= 0.0
            || !(1..=64).contains(&light.samples)
        {
            return Err(format!(
                "baked rectangular area light on entity {:?} has invalid parameters",
                actor.entity(),
            )
            .into());
        }
        let transform = engine
            .raw_world()
            .get::<GlobalTransform>(actor.entity())
            .cloned()
            .or_else(|| {
                engine
                    .raw_world()
                    .get::<Transform>(actor.entity())
                    .map(|transform| GlobalTransform::from(transform))
            })
            .unwrap_or_default();
        if !transform.translation.is_finite()
            || !transform.rotation.is_finite()
            || !transform.scale.is_finite()
        {
            return Err("baked rectangular area light has a non-finite transform".into());
        }
        let axis_u = (transform.rotation * Vec3::X).normalize_or_zero();
        let axis_v = (transform.rotation * Vec3::Z).normalize_or_zero();
        let normal = (transform.rotation * Vec3::Y).normalize_or_zero();
        if axis_u.length_squared() < 0.5
            || axis_v.length_squared() < 0.5
            || normal.length_squared() < 0.5
        {
            return Err("baked rectangular area light has an invalid rotation".into());
        }
        out.push(BakeRectAreaLight {
            entity: actor.entity(),
            center: transform.translation,
            axis_u,
            axis_v,
            normal,
            width: light.width * transform.scale.x.abs().max(0.0001),
            height: light.height * transform.scale.z.abs().max(0.0001),
            color: light.color,
            intensity: light.intensity,
            samples: light.samples,
            two_sided: light.two_sided,
        });
    }
    Ok(out)
}

pub(super) fn validate_bake_config(config: &BakedLightingBakeConfig) -> Result<(), Box<dyn Error>> {
    const MAX_PROBE_AXIS: u32 = 256;
    const MAX_TOTAL_PROBES: usize = 262_144;

    if config.source_name.trim().is_empty() {
        return Err("source_name cannot be empty".into());
    }
    if !(4..=512).contains(&config.lightmap_resolution) {
        return Err("lightmap_resolution must be between 4 and 512".into());
    }
    if config.atlas_padding > 64 {
        return Err("atlas_padding cannot exceed 64 texels".into());
    }
    if config.lightmap_filter_radius > 8 {
        return Err("lightmap_filter_radius cannot exceed 8 texels".into());
    }
    if !config.lightmap_texels_per_unit.is_finite()
        || !(0.0..=128.0).contains(&config.lightmap_texels_per_unit)
    {
        return Err("lightmap_texels_per_unit must be finite and between 0 and 128".into());
    }
    if config
        .probe_counts
        .iter()
        .any(|count| *count == 0 || *count > MAX_PROBE_AXIS)
    {
        return Err("each probe_counts axis must be between 1 and 256".into());
    }
    let probe_count = (config.probe_counts[0] as usize)
        .checked_mul(config.probe_counts[1] as usize)
        .and_then(|value| value.checked_mul(config.probe_counts[2] as usize))
        .ok_or("probe count overflow")?;
    if probe_count > MAX_TOTAL_PROBES {
        return Err(format!(
            "probe grid contains {probe_count} probes; maximum is {MAX_TOTAL_PROBES}"
        )
        .into());
    }
    if !(8..=4096).contains(&config.probe_rays) {
        return Err("probe_rays must be between 8 and 4096".into());
    }
    if !config.probe_bounds_padding.is_finite() || config.probe_bounds_padding < 0.0 {
        return Err("probe_bounds_padding must be finite and non-negative".into());
    }
    if !config.environment_radiance.is_finite()
        || config.environment_radiance.min_element() < 0.0
    {
        return Err("environment_radiance must be finite and non-negative".into());
    }
    if !(1..=8).contains(&config.indirect_bounces) {
        return Err("indirect_bounces must be between 1 and 8".into());
    }
    if !config.indirect_bounce_decay.is_finite()
        || !(0.0..=0.95).contains(&config.indirect_bounce_decay)
    {
        return Err("indirect_bounce_decay must be finite and between 0 and 0.95".into());
    }
    for (name, value) in [
        ("indirect_intensity", config.indirect_intensity),
        ("lightmap_intensity", config.lightmap_intensity),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("{name} must be finite and non-negative").into());
        }
    }
    if !config.max_baked_radiance.is_finite()
        || config.max_baked_radiance <= 0.0
        || config.max_baked_radiance > 65_504.0
    {
        return Err("max_baked_radiance must be finite and in 0..=65504 for RGBA16F".into());
    }
    if !config.surface_bias.is_finite() || config.surface_bias <= 0.0 {
        return Err("surface_bias must be finite and positive".into());
    }
    Ok(())
}
