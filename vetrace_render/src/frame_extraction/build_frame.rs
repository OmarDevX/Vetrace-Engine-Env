use super::*;

pub fn build_render_frame(engine: &Engine) -> RenderFrame {
    let settings = engine
        .get_resource::<RenderSettings>()
        .cloned()
        .unwrap_or_default();
    let max_reflection_capture_resolution = settings
        .reflection_max_capture_resolution
        .clamp(32, 1024)
        .next_power_of_two()
        .min(1024);
    let assets = engine.get_resource::<RenderAssets>();
    let baked_lighting = engine.get_resource::<BakedLightingScene>();
    let camera = engine.get_resource::<Camera>().cloned().unwrap_or_default();
    #[cfg(feature = "render_2d")]
    let camera_2d = engine.get_resource::<Camera2D>().cloned().unwrap_or_default();
    let post_processing = engine
        .get_resource::<PostProcessing>()
        .cloned()
        .unwrap_or_default();
    let environment = extract_environment(engine, assets);
    let custom_post_process_passes =
        extract_custom_post_process_passes(engine, &post_processing);
    let mut scene = extract_scene(
        engine,
        assets,
        baked_lighting,
        max_reflection_capture_resolution,
    );

    let (reflection_global_signature, reflection_layer_signatures) = reflection_scene_signatures(
        &scene.objects,
        &scene.directional_lights,
        &scene.point_lights,
        &scene.spot_lights,
        environment.as_ref(),
        assets,
    );
    scene.reflection_probes.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| {
                let da = a.capture_position_world.distance_squared(camera.position);
                let db = b.capture_position_world.distance_squared(camera.position);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.entity.0.cmp(&b.entity.0))
    });

    let egui_overlay = engine
        .get_resource::<EguiOverlayPanel>()
        .cloned()
        .or_else(|| {
            engine
                .get_resource::<DebugTextOverlayPanel>()
                .map(egui_panel_from_debug_overlay)
        });
    let egui_input = engine
        .get_resource::<InputState>()
        .map(egui_input_snapshot_from_input);
    let egui_keyboard_input = engine
        .get_resource::<InputState>()
        .map(egui_keyboard_input_snapshot_from_input);

    RenderFrame {
        settings,
        camera,
        #[cfg(feature = "render_2d")]
        camera_2d,
        render_texture_views: scene.render_texture_views,
        objects: scene.objects,
        sprites: scene.sprites,
        #[cfg(feature = "render_2d")]
        sprites_2d: scene.sprites_2d,
        overlays: scene.overlays,
        #[cfg(feature = "egui_render")]
        world_ui: scene.world_ui,
        #[cfg(feature = "egui_render")]
        screen_ui: scene.screen_ui,
        directional_lights: scene.directional_lights,
        point_lights: scene.point_lights,
        spot_lights: scene.spot_lights,
        environment,
        reflection_probes: scene.reflection_probes,
        reflection_global_signature,
        reflection_layer_signatures,
        atmosphere: scene.atmosphere,
        fog: scene.fog,
        post_processing,
        custom_post_process_passes,
        egui_overlay,
        egui_input,
        egui_keyboard_input,
        #[cfg(feature = "egui_render")]
        egui_tools: engine.get_resource::<EguiToolRegistry>().cloned(),
        #[cfg(feature = "profiler")]
        profiler_report: engine.get_resource::<ProfilerReport>().cloned(),
        #[cfg(feature = "profiler")]
        profiler_ui_settings: engine.get_resource::<ProfilerUiSettings>().cloned(),
    }
}


#[cfg(all(test, feature = "render_2d"))]
mod tests {
    use super::*;
    use glam::{Quat, Vec2, Vec3};
    use vetrace_core::{Plugin, Transform};

    #[test]
    fn render_frame_extracts_and_orders_2d_sprites() {
        let mut engine = Engine::new();
        RenderPlugin::headless().initialize(&mut engine).unwrap();
        Render2dPlugin::new().initialize(&mut engine).unwrap();

        for (name, layer, z) in [("front", 1, 0), ("back", 0, 5), ("middle", 1, -2)] {
            engine
                .spawn_actor(name)
                .with(Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                    scale: Vec3::ONE,
                })
                .with(Sprite2D {
                    size: Vec2::ONE,
                    ..Sprite2D::default()
                })
                .with(CanvasItem2D {
                    canvas_layer: layer,
                    z_index: z,
                    ..CanvasItem2D::default()
                })
                .build();
        }

        let frame = build_render_frame(&engine);
        let order = frame
            .sprites_2d
            .iter()
            .map(|sprite| (sprite.canvas.canvas_layer, sprite.canvas.z_index))
            .collect::<Vec<_>>();
        assert_eq!(order, vec![(0, 5), (1, -2), (1, 0)]);
    }
}
