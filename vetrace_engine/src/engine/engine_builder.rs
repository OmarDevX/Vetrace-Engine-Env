use crate::assets::AssetManager;
use crate::ecs::World;
use crate::engine::core::EngineCore;
use crate::engine::engine_v2::Engine;
use crate::engine::managers::{
    ComponentManager, EventManager, InputManager, RenderingManager, ScriptingManager, UIManager,
};
use crate::engine::physics::PhysicsState;
use crate::engine::SceneManager;
use crate::input::{window::WindowManager, Input};
#[cfg(feature = "use_epi")]
use crate::rendering::EguiRenderer;
use crate::rendering::Renderer;
use crate::scene::scene::Scene;
#[cfg(all(not(feature = "wgpu"), feature = "use_epi"))]
use crate::shared::ShaderVersion;
use crate::systems::free_flight::FreeFlightState;
#[cfg(not(feature = "wgpu"))]
use crate::systems::sprite_render::SpriteRenderSystem;
use egui::Context as EguiContext;
use std::sync::Arc;

/// Builder for creating Engine instances with a fluent API
pub struct EngineBuilder {
    is_2d: bool,
    // Add other configuration options as needed
}

impl EngineBuilder {
    /// Create a new EngineBuilder
    pub fn new() -> Self {
        Self { is_2d: false }
    }

    /// Set whether the engine should run in 2D mode
    pub fn with_2d_mode(mut self, is_2d: bool) -> Self {
        self.is_2d = is_2d;
        self
    }

    /// Build the Engine instance
    ///
    /// This method creates all the necessary managers and components,
    /// then assembles them into a complete Engine instance.
    pub fn build(self) -> Result<Engine, Box<dyn std::error::Error>> {
        // Initialize SDL
        let sdl_context = sdl2::init()?;

        // Create window first so it can be shared between managers
        let window = WindowManager::new(sdl_context.clone());

        // Create core components
        let world = World::new();
        let core = EngineCore::new();
        let physics = PhysicsState::new();
        let assets = Arc::new(AssetManager::new("assets"));

        // Create managers
        let events = EventManager::new(self.is_2d);
        let scripting = ScriptingManager::new();
        let components = ComponentManager::new();

        // Initialize rendering
        let rendering = self.create_rendering_manager(&window)?;

        // Initialize input
        let input = self.create_input_manager(sdl_context, window)?;

        // Initialize UI
        let ui = self.create_ui_manager()?;

        Ok(Engine::new(
            core, world, physics, assets, rendering, input, scripting, components, events, ui,
        ))
    }

    /// Create the rendering manager using the provided window
    fn create_rendering_manager(
        &self,
        window: &WindowManager,
    ) -> Result<RenderingManager, Box<dyn std::error::Error>> {
        let (width, height) = window.get_size();
        let renderer = Renderer::new(&window.window, width, height, self.is_2d);
        let scene = Scene::new();
        let egui_ctx = EguiContext::default();
        #[cfg(all(feature = "wgpu", feature = "use_epi"))]
        let egui_renderer = EguiRenderer::new(
            renderer.device(),
            renderer.surface_format(),
            1.0,
            (width as u32, height as u32),
        );
        #[cfg(all(not(feature = "wgpu"), feature = "use_epi"))]
        let egui_renderer = EguiRenderer::new(&window.window, 1.0, ShaderVersion::Default);
        #[cfg(not(feature = "wgpu"))]
        let sprite_renderer = SpriteRenderSystem::new();
        Ok(RenderingManager::new(
            renderer,
            scene,
            egui_ctx,
            #[cfg(feature = "use_epi")]
            egui_renderer,
            #[cfg(not(feature = "wgpu"))]
            sprite_renderer,
        ))
    }

    /// Create the input manager from SDL context and window
    fn create_input_manager(
        &self,
        sdl_context: sdl2::Sdl,
        window: WindowManager,
    ) -> Result<InputManager, Box<dyn std::error::Error>> {
        let input = Input::new();
        let free_flight = FreeFlightState::new();
        Ok(InputManager::new(input, window, sdl_context, free_flight))
    }

    /// Create the UI manager
    fn create_ui_manager(&self) -> Result<UIManager, Box<dyn std::error::Error>> {
        Ok(UIManager::new(SceneManager::new()))
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a new Engine with default settings
pub fn create_engine() -> Result<Engine, Box<dyn std::error::Error>> {
    EngineBuilder::new().build()
}

/// Convenience function to create a new 2D Engine
pub fn create_2d_engine() -> Result<Engine, Box<dyn std::error::Error>> {
    EngineBuilder::new().with_2d_mode(true).build()
}
