use vetrace_engine::Engine;
use vetrace_engine::components::components::Animation;

/// Handles animation setup and debugging for the PBR cat example
pub struct AnimationManager;

impl AnimationManager {
    /// Print detailed information about an animation clip
    pub fn debug_animation_info(engine: &Engine, animation_name: &str) {
        if let Some(clip) = engine.assets.get_animation(animation_name) {
            println!("Animation '{}' details:", animation_name);
            println!("  Duration: {:.2}s", clip.duration);
            println!("  Channels: {}", clip.channels.len());
            
            for (i, channel) in clip.channels.iter().enumerate() {
                match channel {
                    vetrace_engine::assets::AnimationChannel::Translation(keyframes) => {
                        println!("  Channel {}: Translation with {} keyframes", i, keyframes.len());
                    }
                    vetrace_engine::assets::AnimationChannel::Rotation(keyframes) => {
                        println!("  Channel {}: Rotation with {} keyframes", i, keyframes.len());
                    }
                    vetrace_engine::assets::AnimationChannel::Scale(keyframes) => {
                        println!("  Channel {}: Scale with {} keyframes", i, keyframes.len());
                    }
                    vetrace_engine::assets::AnimationChannel::MorphTargetWeights(keyframes) => {
                        println!("  Channel {}: MorphTargetWeights with {} keyframes", i, keyframes.len());
                    }
                }
            }
        } else {
            println!("Animation '{}' not found", animation_name);
        }
    }
    
    /// Setup animation on an entity with the given parameters
    pub fn setup_animation(
        engine: &mut Engine, 
        object_id: u32, 
        animation_name: &str,
        translation_scale: f32
    ) -> Result<(), String> {
        // Validate that the animation exists
        if !engine.assets.get_animation(animation_name).is_some() {
            return Err(format!("Animation '{}' not found in asset manager", animation_name));
        }
        
        // Convert the object_id (which is actually an entity ID) back to an entity
        let entity = vetrace_engine::ecs::Entity(object_id);
        
        // Get the animation component and configure it
        if let Some(mut anim) = engine.world.get_mut::<Animation>(entity) {
            anim.clip = animation_name.to_string();
            anim.time = 0.0; // restart when switching
            anim.translation_scale = translation_scale;
            
            println!("Successfully configured animation: {} (translation scale: {})", 
                animation_name, translation_scale);
            Ok(())
        } else {
            Err(format!("Entity {:?} does not have an Animation component", entity))
        }
    }
    
    /// Setup the first available animation with default settings
    pub fn setup_first_available_animation(engine: &mut Engine, object_id: u32) -> Result<(), String> {
        let animations = engine.assets.animation_names();
        
        if animations.is_empty() {
            return Err("No animations available".to_string());
        }
        
        let first_animation = &animations[0];
        println!("Available animations: {:?}", animations);
        
        // Debug the animation details
        Self::debug_animation_info(engine, first_animation);
        
        // Setup with a reasonable translation scale (10% of original)
        Self::setup_animation(engine, object_id, first_animation, 0.1)
    }
}
