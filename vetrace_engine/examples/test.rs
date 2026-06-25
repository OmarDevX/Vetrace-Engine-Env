use sdl2::keyboard::Keycode;
use vetrace_engine::components::components::{DirectionalLight, Sprite3D, Transform};
use vetrace_engine::ecs::Entity;
use vetrace_engine::engine::Engine;
use vetrace_engine::rendering::TextureStorage;
use vetrace_engine::Behaviour;

struct PlayerController {
    ent: Entity,
}

impl Behaviour for PlayerController {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut dir = [0.0f32, 0.0];
        if engine.input.is_key_down(Keycode::W) {
            dir[1] += 1.0;
        }
        if engine.input.is_key_down(Keycode::S) {
            dir[1] -= 1.0;
        }
        if engine.input.is_key_down(Keycode::D) {
            dir[0] += 1.0;
        }
        if engine.input.is_key_down(Keycode::A) {
            dir[0] -= 1.0;
        }
        let len = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        if len > 0.0 {
            dir[0] /= len;
            dir[1] /= len;
        }
        if let Some(mut t) = engine.world.get_mut::<Transform>(self.ent) {
            let speed = 2.0;
            t.position[0] += dir[0] * speed * delta;
            t.position[1] += dir[1] * speed * delta;
        }
    }
}

fn main() {
    // Start the engine in 2D mode
    let mut engine = Engine::new(true);

    engine.sky_color = [135.0, 206.0, 235.0];

    // Load the tree texture used for all sprites
    let mut textures = TextureStorage::new();
    let tree_tex = textures.load_texture("assets/textures/tree.jpg");

    // Player sprite in the center
    let player = engine.spawn_empty("player");
    engine.world.insert(
        player,
        Transform {
            size: [1.0, 1.0, 1.0],
            ..Default::default()
        },
    );
    engine.world.insert(
        player,
        Sprite3D {
            texture: tree_tex.clone(),
            size: [1.0, 1.0],
            facing_camera: true,
            double_sided: true,
        },
    );

    // Floor sprite stretched horizontally
    let floor = engine.spawn_empty("floor");
    engine.world.insert(
        floor,
        Transform {
            position: [0.0, -2.0, 0.0],
            size: [6.0, 1.0, 1.0],
            ..Default::default()
        },
    );
    engine.world.insert(
        floor,
        Sprite3D {
            texture: tree_tex.clone(),
            size: [1.0, 1.0],
            facing_camera: true,
            double_sided: true,
        },
    );

    // Left wall
    let left = engine.spawn_empty("left");
    engine.world.insert(
        left,
        Transform {
            position: [-3.0, 0.0, 0.0],
            size: [1.0, 4.0, 1.0],
            ..Default::default()
        },
    );
    engine.world.insert(
        left,
        Sprite3D {
            texture: tree_tex.clone(),
            size: [1.0, 1.0],
            facing_camera: true,
            double_sided: true,
        },
    );

    // Right wall
    let right = engine.spawn_empty("right");
    engine.world.insert(
        right,
        Transform {
            position: [3.0, 0.0, 0.0],
            size: [1.0, 4.0, 1.0],
            ..Default::default()
        },
    );
    engine.world.insert(
        right,
        Sprite3D {
            texture: tree_tex,
            size: [1.0, 1.0],
            facing_camera: true,
            double_sided: true,
        },
    );

    // Basic directional light so the sprite is lit and casts shadows
    let light = engine.spawn_empty("light");
    engine.world.insert(
        light,
        DirectionalLight {
            direction: [-0.5, -1.0, -0.5],
            color: [255.0, 255.0, 255.0],
            intensity: 1.0,
            ..Default::default()
        },
    );

    // run engine with simple behaviour to move the player sprite
    engine.run_with_behaviour(true, PlayerController { ent: player });
}