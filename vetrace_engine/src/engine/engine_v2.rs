use std::sync::Arc;
use glam::{Quat, Vec3};

use crate::assets::AssetManager;
use crate::ecs::World;
use crate::engine::core::EngineCore;
use crate::engine::physics::PhysicsState;
use crate::engine::managers::{
    RenderingManager, InputManager, ScriptingManager, 
    ComponentManager, EventManager, UIManager
};

/// Camera information structure
#[derive(Clone, Copy)]
pub struct CameraInfo {
    pub position: Vec3,
    pub orientation: Quat,
    pub fov: f32,
    pub velocity: Vec3,
}

impl Default for CameraInfo {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            fov: 60.0_f32.to_radians(),
            velocity: Vec3::ZERO,
        }
    }
}

/// The main Engine struct - now clean and modular!
/// 
/// This is the new, refactored Engine that separates concerns into specialized managers.
/// Each manager handles a specific aspect of the engine, making the code much more
/// maintainable and easier to understand.
pub struct Engine {
    // Core engine systems
    pub core: EngineCore,
    pub world: World,
    pub physics: PhysicsState,
    pub assets: Arc<AssetManager>,

    // Specialized managers
    pub rendering: RenderingManager,
    pub input: InputManager,
    pub scripting: ScriptingManager,
    pub components: ComponentManager,
    pub events: EventManager,
    pub ui: UIManager,
}

impl Engine {
    /// Create a new Engine instance
    pub fn new(
        core: EngineCore,
        world: World,
        physics: PhysicsState,
        assets: Arc<AssetManager>,
        rendering: RenderingManager,
        input: InputManager,
        scripting: ScriptingManager,
        components: ComponentManager,
        events: EventManager,
        ui: UIManager,
    ) -> Self {
        Self {
            core,
            world,
            physics,
            assets,
            rendering,
            input,
            scripting,
            components,
            events,
            ui,
        }
    }

    // Convenience methods for common operations

    /// Check if the engine is running
    pub fn is_running(&self) -> bool {
        self.events.is_running()
    }

    /// Stop the engine
    pub fn stop(&mut self) {
        self.events.set_running(false);
    }

    /// Check if the engine is paused
    pub fn is_paused(&self) -> bool {
        self.events.is_paused()
    }

    /// Pause/unpause the engine
    pub fn set_paused(&mut self, paused: bool) {
        self.events.set_paused(paused);
    }

    /// Get the renderer reference
    pub fn renderer(&self) -> &crate::rendering::Renderer {
        self.rendering.renderer()
    }

    /// Get mutable renderer reference
    pub fn renderer_mut(&mut self) -> &mut crate::rendering::Renderer {
        self.rendering.renderer_mut()
    }

    /// Get the scene reference
    pub fn scene(&self) -> &crate::scene::scene::Scene {
        self.rendering.scene()
    }

    /// Get mutable scene reference
    pub fn scene_mut(&mut self) -> &mut crate::scene::scene::Scene {
        self.rendering.scene_mut()
    }

    /// Get input reference
    pub fn input(&self) -> &crate::input::Input {
        self.input.input()
    }

    /// Get mutable input reference
    pub fn input_mut(&mut self) -> &mut crate::input::Input {
        self.input.input_mut()
    }

    /// Get window manager reference
    pub fn window(&self) -> &crate::input::window::WindowManager {
        self.input.window()
    }

    /// Get mutable window manager reference
    pub fn window_mut(&mut self) -> &mut crate::input::window::WindowManager {
        self.input.window_mut()
    }

    /// Spawn a new empty entity with a name
    pub fn spawn_empty(&mut self, name: &str) -> crate::ecs::Entity {
        let entity = self.world.spawn();
        self.world.insert(
            entity,
            crate::components::components::Metadata {
                name: name.into(),
                tags: Vec::new(),
            },
        );
        entity
    }

    /// Find entity by object ID
    pub fn find_entity_by_object_id(&self, object_id: u32) -> Option<crate::ecs::Entity> {
        self.core.find_entity_by_object_id(object_id)
    }
}

/// Legacy compatibility - this will help with migration
pub type LegacyEngine = crate::engine::engine::Engine;
