use sdl2::keyboard::Keycode;
use vetrace_engine::app::{app, App};
use vetrace_engine::components::components::Transform;
use vetrace_engine::Engine;

/// Simple game application demonstrating the application framework API.
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

        for i in 0..5 {
            let entity = engine.spawn_empty(&format!("demo_entity_{i}"));
            engine.world.insert(
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
        if engine.input.is_key_down(Keycode::Escape) {
            println!("Escape pressed; close the window to exit.");
        }

        let entities: Vec<_> = engine.world.entities().iter().copied().collect();
        for entity in entities {
            if let Some(transform) = engine.get_component_mut_entity::<Transform>(entity) {
                transform.orientation[1] += delta_time;
            }
        }
    }

    fn cleanup(&mut self, _engine: &mut Engine) {
        println!("Cleaning up Simple Game!");
        println!("Final entity count: {}", self.entity_count);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Clean Engine Demo");

    app()
        .with_title("Clean Engine Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(SimpleGame::new())
}
