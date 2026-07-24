use super::*;

pub(super) fn extract_entity_ui(
    engine: &Engine,
    entity: Entity,
    transform: &GlobalTransform,
    scene: &mut SceneExtraction,
) {
    #[cfg(not(feature = "egui_render"))]
    let _ = (engine, entity, transform, scene);

    #[cfg(feature = "egui_render")]
    {
        if let Some(world_space) = engine.raw_world().get::<vetrace_ui::UIWorldSpace>(entity) {
            if world_space.visible {
                let placement = RenderWorldUiPlacement {
                    screen_offset_px: world_space.screen_offset_px,
                    size_px: world_space.size_px,
                    max_distance: world_space.max_distance,
                    z_order: world_space.z_order,
                    anchor: world_space.anchor,
                    background: world_space.background,
                    background_alpha: world_space.background_alpha,
                    padding_px: world_space.padding_px,
                };
                push_world_ui_elements(
                    engine,
                    entity,
                    transform.translation,
                    placement,
                    &mut scene.world_ui,
                );
            }
        }

        if engine.raw_world().has::<vetrace_ui::UIScreenSpace>(entity) {
            if let Some(rect) = engine.raw_world().get::<ScreenSpaceRect>(entity).cloned() {
                push_screen_ui_elements(engine, entity, rect, &mut scene.screen_ui);
            }
        }
    }
}
