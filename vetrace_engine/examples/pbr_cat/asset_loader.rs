use vetrace_engine::Engine;

/// Handles loading and managing assets for the PBR cat example
pub struct AssetLoader;

impl AssetLoader {
    /// Load the cat GLTF model and return its object ID
    pub fn load_cat_model(engine: &mut Engine) -> Result<u32, Box<dyn std::error::Error>> {
        println!("Loading cat model...");
        
        let assets = engine.assets.clone();
        let cat_id = assets.load_gltf_pbr(engine, "oii_cat/scene.gltf")?;
        
        println!("Cat model loaded successfully with ID: {}", cat_id);
        Ok(cat_id)
    }
    
    /// Get a list of all available animations
    pub fn list_available_animations(engine: &Engine) -> Vec<String> {
        engine.assets.animation_names()
    }
    
    /// Check if a specific animation exists
    pub fn animation_exists(engine: &Engine, name: &str) -> bool {
        engine.assets.get_animation(name).is_some()
    }
}
