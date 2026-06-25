use vetrace_engine::Engine;
use vetrace_engine::components::components::{
    CameraAttachment, DirectionalLight, FreeFlightControls, Transform,
};
use vetrace_engine::ecs::Entity;

/// Handles scene setup including camera, lighting, and environment
pub struct SceneSetup;

impl SceneSetup {
    /// Create and configure the main camera
    pub fn setup_camera(engine: &mut Engine) -> Entity {
        let camera_entity = engine.spawn_empty("camera");
        
        // Position camera to get a good view of the cat
        engine.world.insert(
            camera_entity,
            Transform {
                position: [0.0, 2.0, 5.0], // Back and up to see the cat better
                ..Default::default()
            },
        );
        
        // Add camera attachment and flight controls
        engine.world.insert(camera_entity, CameraAttachment::default());
        engine.world.insert(camera_entity, FreeFlightControls::default());
        
        println!("Camera setup complete");
        camera_entity
    }
    
    /// Setup basic lighting for the scene
    pub fn setup_lighting(engine: &mut Engine, camera_entity: Entity) {
        engine.world.insert(
            camera_entity,
            DirectionalLight {
                direction: [-1.0, -1.0, -1.0], // Light coming from upper-left
                color: [255.0, 255.0, 255.0],  // White light
                intensity: 1.0,                 // Full intensity
                ..Default::default()
            },
        );
        
        println!("Lighting setup complete");
    }
    
    /// Setup the complete scene with camera and lighting
    pub fn setup_complete_scene(engine: &mut Engine) -> Entity {
        let camera_entity = Self::setup_camera(engine);
        Self::setup_lighting(engine, camera_entity);
        camera_entity
    }
}
