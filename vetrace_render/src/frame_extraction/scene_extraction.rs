use super::*;

#[derive(Default)]
pub(super) struct SceneExtraction {
    pub(super) objects: Vec<RenderObject>,
    pub(super) render_texture_views: Vec<RenderTextureView>,
    pub(super) sprites: Vec<RenderSprite>,
    #[cfg(feature = "render_2d")]
    pub(super) sprites_2d: Vec<RenderSprite2D>,
    pub(super) overlays: Vec<RenderOverlayRect>,
    #[cfg(feature = "egui_render")]
    pub(super) world_ui: Vec<RenderWorldUiElement>,
    #[cfg(feature = "egui_render")]
    pub(super) screen_ui: Vec<RenderScreenUiElement>,
    pub(super) directional_lights: Vec<RenderDirectionalLight>,
    pub(super) point_lights: Vec<RenderPointLight>,
    pub(super) spot_lights: Vec<RenderSpotLight>,
    pub(super) reflection_probes: Vec<RenderReflectionProbe>,
    pub(super) atmosphere: Option<Atmosphere>,
    pub(super) fog: Option<VolumetricFog>,
}

pub(super) fn extract_scene(
    engine: &Engine,
    assets: Option<&RenderAssets>,
    baked_lighting: Option<&BakedLightingScene>,
    max_reflection_capture_resolution: u32,
) -> SceneExtraction {
    let mut scene = SceneExtraction::default();

    for entity in engine.raw_world().entities() {
        let transform = global_transform_for(engine, entity);
        extract_entity_view_lights_and_environment(
            engine,
            entity,
            &transform,
            assets,
            max_reflection_capture_resolution,
            &mut scene,
        );
        extract_entity_ui(engine, entity, &transform, &mut scene);
        #[cfg(feature = "render_2d")]
        extract_entity_2d(engine, entity, &transform, &mut scene);
        extract_entity_renderables(
            engine,
            entity,
            transform,
            assets,
            baked_lighting,
            &mut scene,
        );
    }

    #[cfg(feature = "render_2d")]
    scene.sprites_2d.sort_by(|a, b| {
        a.canvas
            .canvas_layer
            .cmp(&b.canvas.canvas_layer)
            .then_with(|| a.canvas.z_index.cmp(&b.canvas.z_index))
            .then_with(|| a.entity.0.cmp(&b.entity.0))
    });

    scene.render_texture_views.sort_by(|a, b| {
        a.order
            .cmp(&b.order)
            .then_with(|| a.target_name.cmp(&b.target_name))
    });
    scene
}
