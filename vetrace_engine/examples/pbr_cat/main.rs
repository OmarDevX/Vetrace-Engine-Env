mod asset_loader;
mod animation_manager;
mod scene_setup;

use vetrace_engine::Engine;
use asset_loader::AssetLoader;
use animation_manager::AnimationManager;
use scene_setup::SceneSetup;

fn main() {
    println!("Starting PBR Cat Example");

    // Initialize the engine
    let mut engine = Engine::new(false);
    
    // Load the cat model
    let cat_id = AssetLoader::load_cat_model(&mut engine)
        .expect("Failed to load cat model");
    
    // Setup animations
    if let Err(e) = AnimationManager::setup_first_available_animation(&mut engine, cat_id) {
        println!("Animation setup failed: {}", e);
    }
    
    // Setup the scene (camera and lighting)
    SceneSetup::setup_complete_scene(&mut engine);

    // Optional: Scale up the cat to make it more visible
    let cat_entity = vetrace_engine::ecs::Entity(cat_id);
    if let Some(mut transform) = engine.world.get_mut::<vetrace_engine::components::components::Transform>(cat_entity) {
        // Scale up the cat 3x to make it more prominent
        transform.size = [
            transform.size[0],
            transform.size[1],
            transform.size[2],
        ];
        println!("Scaled cat to size: {:?}", transform.size);
    }

    println!("Scene setup complete. Starting engine...");

    // Run the engine
    engine.run(true);
}
