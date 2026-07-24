// Per-object runtime binding data and probe/lightmap selection.

use super::*;

#[derive(Clone, Debug)]
pub(crate) struct RenderBakedLightmap {
    pub atlas: std::sync::Arc<BakedLightmapAtlas>,
    pub region: BakedLightmapRegion,
    pub debug_mode: BakedLightingDebugMode,
    pub runtime_mode: BakedLightingRuntimeMode,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RenderBakedProbes {
    pub sample: BakedProbeSample,
    pub intensity: f32,
    pub debug_mode: BakedLightingDebugMode,
}

pub(crate) fn render_baked_lighting_for_object(
    scene: Option<&BakedLightingScene>,
    object: &RenderObject,
    assets: Option<&RenderAssets>,
    wants_lightmap: bool,
    wants_probes: Option<f32>,
) -> (Option<RenderBakedLightmap>, Option<RenderBakedProbes>) {
    let Some(scene) = scene.filter(|scene| scene.enabled) else { return (None, None); };
    let lightmap = if wants_lightmap {
        let key = baked_object_key(object, assets);
        scene
            .lightmaps
            .get(&key)
            .copied()
            .zip(scene.atlas.as_ref())
            .map(|(region, atlas)| RenderBakedLightmap {
                atlas: atlas.clone(),
                region,
                debug_mode: scene.debug_mode,
                runtime_mode: scene.runtime_mode,
            })
    } else {
        None
    };
    let probe_intensity = wants_probes
        .filter(|intensity| *intensity > 0.0)
        .or_else(|| {
            (wants_lightmap && scene.debug_mode == BakedLightingDebugMode::Probes)
                .then_some(1.0)
        });
    let active_probe_grid = match scene.runtime_mode {
        BakedLightingRuntimeMode::BakedOnly => &scene.probes,
        BakedLightingRuntimeMode::HybridRealtimeDirect => &scene.indirect_probes,
    };
    let probes = probe_intensity.map(|intensity| RenderBakedProbes {
        sample: active_probe_grid.sample(object.transform.translation),
        intensity,
        debug_mode: scene.debug_mode,
    });
    (lightmap, probes)
}
