use vetrace_engine::Engine;
use vetrace_engine::app::App;
use vetrace_engine::components::components::Transform;
use vetrace_engine::rendering::RenderParams;
/// Simple game application demonstrating the new clean engine API
struct SimpleGame {
    entity_count: usize,
}

impl SimpleGame {
    fn new() -> Self {
        Self { entity_count: 0 }
    }
}

impl App for SimpleGame {
    fn setup(&mut self, engine: &mut Engine) {
        println!("Setting up Simple Game!");

        // Add a simple system that prints every 60 frames
        let mut frame_counter = 0;

        // Spawn some test entities
        for i in 0..5 {
            let entity = engine.spawn_entity();
            engine.add_component(
                entity,
                Transform {
                    position: [i as f32, 0.0, 0.0],
                    ..Default::default()
                },
            );
            self.entity_count += 1;
        }

        println!("Spawned {} entities", self.entity_count);
    }

    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        // Application-specific update logic
        if engine.input.is_key_pressed("Escape") {
            println!("Escape pressed, exiting...");
            engine.stop();
        }

        // Example: Rotate all entities
        let entities: Vec<_> = engine.world.entities().iter().copied().collect();
        for entity in entities {
            if let Some(transform) = engine.get_component_mut::<Transform>(entity) {
                transform.orientation[1] += delta_time; // Rotate around Y axis
            }
        }
    }

    fn render(&mut self, engine: &mut Engine) {
        // Create basic render parameters
        let render_params = RenderParams {
            camera_pos: [0.0, 0.0, 5.0],
            camera_front: [0.0, 0.0, -1.0],
            camera_up: [0.0, 1.0, 0.0],
            camera_right: [1.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            fov: 60.0,
            num_objects: 0,
            current_time: 0.0,
            skycolor: [0.5, 0.7, 1.0],
            is_fisheye: 0,
            selected_index: 0,
            max_bounces: 8,
            light_samples: 1,
            dir_shadow_samples: 1,
            raytraced_shadows_enabled: 1,
            shadow_quality: 2,
            max_shadow_rays: 2,
            emissive_shadow_samples: 1,
            directional_shadow_samples: 1,
            cloud_object_shadows_enabled: 1,
            inv_view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            prev_view_proj: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            gi_quality: 1,
            gi_debug_mode: 0,
            gi_mode: 0,
            dir_light_dir: [0.0, -1.0, 0.0],
            dir_light_color: [1.0, 1.0, 1.0],
            dir_light_intensity: 1.0,
            sky_occlusion: 0.0,
            dof_aperture: 0.0,
            dof_focus_dist: 1.0,
            dof_enable: 0,
            atmos: Vec::new(),
            atmosphere: 0,
            atmosphere_mode: 0,
            cloud_history_weight: 0.88,
            cloud_sample_count: 0,
            cloud_temporal_quality: 1,
            cloud_shadow_mode: 0,
            atmosphere_sun_controls: [0.00465, 1.0, 1.0, 0.0],
            renderer_mode: vetrace_engine::rendering::renderer::RendererMode::RasterGame,
            clouds: Vec::new(),
        };

        // Render with empty sprite and PBR data for now
        engine.renderer_mut().render(&render_params, &[], &[], None);
    }

    fn cleanup(&mut self, engine: &mut Engine) {
        println!("Cleaning up Simple Game!");
        println!("Final entity count: {}", self.entity_count);
    }

    fn should_continue(&self, engine: &Engine) -> bool {
        // Continue running unless explicitly stopped
        engine.is_running()
    }
}

fn main() -> anyhow::Result<()> {
    println!("Starting Clean Engine Demo");

    // Create engine with fluent builder API
    let engine = Engine::builder()
        .with_window_title("Clean Engine Demo")
        .with_window_size(1280, 720)
        .with_target_fps(60)
        .build()?;

    // Run the application
    engine.run(SimpleGame::new())
}
