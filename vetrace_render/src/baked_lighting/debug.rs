// Runtime debug modes and probe marker visualization.

/// Returns the currently active baked-lighting debug visualization.
use super::*;

pub fn baked_lighting_debug_mode(engine: &Engine) -> BakedLightingDebugMode {
    engine
        .get_resource::<BakedLightingScene>()
        .map(|scene| scene.debug_mode)
        .unwrap_or(BakedLightingDebugMode::Off)
}

/// Changes only the runtime visualization; it does not rebake or modify the file.
pub fn set_baked_lighting_debug_mode(engine: &mut Engine, mode: BakedLightingDebugMode) {
    if let Some(scene) = engine.get_resource_mut::<BakedLightingScene>() {
        scene.debug_mode = mode;
    }
    sync_baked_light_probe_debug_markers(engine);
}

/// Returns how static receivers currently combine baked and realtime lighting.
pub fn baked_lighting_runtime_mode(engine: &Engine) -> BakedLightingRuntimeMode {
    engine
        .get_resource::<BakedLightingScene>()
        .map(|scene| scene.runtime_mode)
        .unwrap_or_default()
}

/// Switches runtime composition only. No bake data is changed.
pub fn set_baked_lighting_runtime_mode(engine: &mut Engine, mode: BakedLightingRuntimeMode) {
    if let Some(scene) = engine.get_resource_mut::<BakedLightingScene>() {
        scene.runtime_mode = mode;
    }
    sync_baked_light_probe_debug_markers(engine);
}

fn clear_baked_light_probe_debug_markers(engine: &mut Engine) {
    let markers = engine
        .actors_with::<BakedLightProbeDebugMarker>()
        .into_iter()
        .map(|(actor, _)| actor)
        .collect::<Vec<_>>();
    for marker in markers {
        marker.despawn(engine);
    }
}

fn sync_baked_light_probe_debug_markers(engine: &mut Engine) {
    clear_baked_light_probe_debug_markers(engine);
    let Some(grid) = engine
        .get_resource::<BakedLightingScene>()
        .filter(|scene| scene.enabled && scene.debug_mode == BakedLightingDebugMode::Probes)
        .map(|scene| match scene.runtime_mode {
            BakedLightingRuntimeMode::BakedOnly => scene.probes.clone(),
            BakedLightingRuntimeMode::HybridRealtimeDirect => scene.indirect_probes.clone(),
        })
    else {
        return;
    };

    let axis_spacing = |extent: f32, count: u32| {
        if count > 1 { extent / count.saturating_sub(1) as f32 } else { extent }
    };
    let extent = (grid.max - grid.min).max(Vec3::splat(0.001));
    let spacing = Vec3::new(
        axis_spacing(extent.x, grid.counts[0]),
        axis_spacing(extent.y, grid.counts[1]),
        axis_spacing(extent.z, grid.counts[2]),
    );
    let marker_size = (spacing.min_element() * 0.14).clamp(0.10, 0.28);

    for z in 0..grid.counts[2] {
        for y in 0..grid.counts[1] {
            for x in 0..grid.counts[0] {
                let fraction = |index: u32, count: u32| {
                    if count <= 1 { 0.5 } else { index as f32 / count.saturating_sub(1) as f32 }
                };
                let t = Vec3::new(
                    fraction(x, grid.counts[0]),
                    fraction(y, grid.counts[1]),
                    fraction(z, grid.counts[2]),
                );
                let position = grid.min + extent * t;
                let sample = grid.samples[(z * grid.counts[1] * grid.counts[0]
                    + y * grid.counts[0] + x) as usize];
                let average = (
                    sample.irradiance_for_normal(Vec3::X)
                    + sample.irradiance_for_normal(Vec3::NEG_X)
                    + sample.irradiance_for_normal(Vec3::Y)
                    + sample.irradiance_for_normal(Vec3::NEG_Y)
                    + sample.irradiance_for_normal(Vec3::Z)
                    + sample.irradiance_for_normal(Vec3::NEG_Z)
                ) / 6.0;
                let mapped = average.max(Vec3::ZERO) / (Vec3::ONE + average.max(Vec3::ZERO));
                let color = mapped.max(Vec3::splat(0.035));
                engine
                    .spawn_actor(format!("Baked Probe Debug {x}:{y}:{z}"))
                    .with(BakedLightProbeDebugMarker)
                    .with(BakedLightProbeReceiver::default())
                    .with(Transform {
                        translation: position,
                        scale: Vec3::splat(marker_size),
                        ..Transform::default()
                    })
                    .with(Shape { primitive: PrimitiveShape::Sphere, size: Vec3::ONE })
                    .with(Material {
                        base_color: color,
                        emissive: color * 2.5,
                        roughness: 0.55,
                        metallic: 0.0,
                        alpha: 0.92,
                        alpha_mode: AlphaMode::Blend,
                        ..Material::default()
                    })
                    .with(Renderable { visible: true, ..Renderable::default() })
                    .build();
            }
        }
    }
}

/// Cycles Off -> Lightmap -> LightmapUv -> Probes -> Off.
pub fn cycle_baked_lighting_debug_mode(engine: &mut Engine) -> BakedLightingDebugMode {
    let next = baked_lighting_debug_mode(engine).next();
    set_baked_lighting_debug_mode(engine, next);
    next
}

pub fn unload_baked_lighting(engine: &mut Engine) {
    clear_baked_light_probe_debug_markers(engine);
    engine.insert_resource(BakedLightingScene::default());
}
