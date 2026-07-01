use vetrace_engine::{Engine, ecs::Entity};
use vetrace_engine::components::components::Animation;

fn main() {
    // Create a 3D engine instance
    let mut engine = Engine::new(false);

    // Load a glTF scene containing a walking animation
    // The path is relative to the engine's asset directory.
    // Replace `player/scene.gltf` with your own model if needed.
    let assets = engine.assets.clone();
    let object_id = assets
        .load_gltf_pbr(&mut engine, "player/scene.gltf")
        .expect("failed to load glTF scene");

    // Choose the animation clip containing "walk" or fall back to the first clip
    let entity = Entity(object_id);
    if let Some(mut anim) = engine.world.get_mut::<Animation>(entity) {
        let clip = assets
            .animation_names()
            .into_iter()
            .find(|n| n.to_lowercase().contains("walk"))
            .unwrap_or_else(|| assets.animation_names().remove(0));
        anim.clip = clip;
        anim.time = 0.0;
    }

    // Run the engine so the walking animation plays
    engine.run(true);
}
