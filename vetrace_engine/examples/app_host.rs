use sdl2::keyboard::Keycode;
#[allow(unused_imports)]
use vetrace_engine::app::{run_app, run_default_app, App, AppConfig};
use vetrace_engine::components::components::Velocity;
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;

struct MyGame {
    player: Option<u32>,
}

impl Default for MyGame {
    fn default() -> Self {
        Self { player: None }
    }
}

impl App for MyGame {
    fn on_start(&mut self, engine: &mut Engine) {
        // Spawn a simple player cube and store its object id
        let mut obj = Object::default();
        obj.position = [0.0, 0.0, 0.0];
        //engine.spawn_cube(obj);
        engine.spawn_object(obj);
        self.player = Some((engine.scene.objects.len() - 1) as u32);
    }

    fn on_update(&mut self, engine: &mut Engine, delta: f32) {
        if let Some(id) = self.player {
            if let Some(entity) = engine.core.find_entity_by_object_id(id) {
                if engine.input.is_key_down(Keycode::A) {
                    if let Some(v) = engine.world.get_mut::<Velocity>(entity) {
                        v.velocity[0] -= 1.0 * delta;
                    } else {
                        engine.world.insert(
                            entity,
                            Velocity {
                                velocity: [-1.0 * delta, 0.0, 0.0],
                                acceleration: [0.0, 0.0, 0.0],
                            },
                        );
                    }
                }
            }
        }
    }

    fn on_render(&mut self, _engine: &mut Engine) {
        // custom rendering or UI could go here
    }
}

fn main() {
    // launch with default configuration (3D + editor UI)
    run_default_app(MyGame::default());
    // or customise like:
    // run_app(MyGame::default(), AppConfig { is_2d: true, enable_editor: false });
}
