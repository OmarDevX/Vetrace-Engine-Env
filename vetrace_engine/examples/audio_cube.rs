use vetrace_engine::components::components::AudioSource;
use vetrace_engine::engine::Engine;
use vetrace_engine::scene::object::Object;

fn main() {
    let mut engine = Engine::new(false);

    // Spawn a simple cube at the origin
    let mut cube = Object::default();
    cube.position = [0.0, 0.0, 0.0];
    //engine.spawn_cube(cube);
    engine.spawn_object(cube);
    let cube_id = (engine.scene.objects.len() - 1) as u32;

    if let Some(entity) = engine.core.find_entity_by_object_id(cube_id) {
        engine.world.insert(
            entity,
            AudioSource {
                clip: Some("assets/sound.wav".to_string()),
                play_on_start: true,
                loop_: true,
                spatial: true,
                pitch: 1.0,
                ..Default::default()
            },
        );
    }

    engine.run(true);
}
