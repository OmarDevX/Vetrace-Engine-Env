use vetrace_engine::engine::Engine;
use vetrace_engine::components::components::{Transform, Sprite3D};
use vetrace_engine::rendering::TextureStorage;

fn main() {
    let mut engine = Engine::new(true);
    let mut textures = TextureStorage::new();
    let handle = textures.load_texture("assets/textures/tree.jpg");

    let entity = engine.spawn_empty("tree");
    engine.world.insert(entity, Transform::default());
    engine.world.insert(entity, Sprite3D {
        texture: handle,
        size: [2.0, 2.0],
        facing_camera: false,
        double_sided: true,
    });

    engine.run(true);
}
