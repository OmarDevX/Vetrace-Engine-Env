use super::Engine;
use crate::components::components::{CameraAttachment, Transform};
use crate::ecs::World;
use crate::engine::core::EngineCore;
use crate::engine::physics::PhysicsState;
use crate::events::{Event as CustomEvent, LuaEvent, SceneEvents};
use crate::input::{window::WindowManager, Input};
use crate::math::vec3_to_array;
#[cfg(feature = "wgpu")]
#[cfg(feature = "wgpu")]
use crate::rendering::wgpu_renderer::PostFxUniforms;
#[cfg(feature = "use_epi")]
use crate::rendering::EguiRenderer;
use crate::rendering::Renderer;
use crate::scene::scene::Scene;
use crate::systems::free_flight::FreeFlightState;
#[cfg(not(feature = "wgpu"))]
use crate::systems::sprite_render::SpriteRenderSystem;
// Note: MainWindow and SandboxWindow have been moved to vetrace_editor crate
#[cfg(not(feature = "wgpu"))]
use crate::shared::ShaderVersion;
use ahash::HashMap;
use ahash::HashMapExt;
use egui::Context as EguiContext;
use std::collections::HashSet;

impl Engine {
    pub fn new(is_2d: bool) -> Self {
        // Disable Wayland's drm-syncobj extension by default to avoid
        // "surface already exists" validation errors on compositors that
        // don't support or mis-handle the protocol. Users can override this
        // by explicitly setting `WGPU_DRM_SYNCOBJ` in their environment
        // prior to launching.
        #[cfg(feature = "wgpu")]
        unsafe {
            if std::env::var("WGPU_DRM_SYNCOBJ").is_err() {
                // SAFETY: setting a process-wide environment variable is required
                // to disable Wayland's drm-syncobj extension when the user has not
                // opted in. This mirrors the behavior of using
                // `WGPU_DRM_SYNCOBJ=0` on the command line.
                std::env::set_var("WGPU_DRM_SYNCOBJ", "0");
            }

            // Some Wayland compositors still advertise the drm-syncobj protocol
            // even when disabled. Falling back to SDL's X11 backend avoids the
            // "surface already exists" panic that results from attempting to
            // configure a Wayland surface in this state.
            if std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("SDL_VIDEODRIVER").is_err()
            {
                std::env::set_var("SDL_VIDEODRIVER", "x11");
            }
        }

        let sdl_context = sdl2::init().unwrap();
        let window = WindowManager::new(sdl_context.clone());
        let (width, height) = window.get_size();

        let renderer = Renderer::new(&window.window, width as i32, height as i32, is_2d);
        let free_flight = FreeFlightState::new();
        let scene = Scene::new();
        let scene_manager = crate::engine::SceneManager::new();
        let input = Input::new();
        // Note: SandboxWindow moved to vetrace_editor crate
        let physics = crate::engine::physics::PhysicsState::new();
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
        let world = World::new();
        let component_factories = HashMap::new();
        let component_adders = HashMap::new();
        let component_removers = HashMap::new();
        let component_editors = HashMap::new();
        let component_checkers = HashMap::new();
        let component_accessors = HashMap::new();
        let assets = std::sync::Arc::new(crate::assets::AssetManager::new("assets"));
        let mut engine = Self {
            renderer,
            scene,
            physics,
            input,
            window,
            running: true,
            sky_color: [135.0, 206.0, 235.0],
            is_fisheye: false,
            // sandbox_window moved to vetrace_editor crate
            egui_ctx,
            #[cfg(feature = "use_epi")]
            egui_renderer,
            #[cfg(not(feature = "wgpu"))]
            sprite_renderer: SpriteRenderSystem::new(),
            egui_events: Vec::new(),
            world,
            free_flight,
            sdl_context,
            behaviours: Vec::new(),
            script_library: HashMap::new(),
            component_behaviours: HashMap::new(),
            component_factories,
            component_adders,
            component_removers,
            component_editors,
            component_checkers,
            component_accessors,
            generated_components: Vec::new(),
            generated_specs: HashMap::new(),
            collision_events: Vec::new(),
            collision_event: CustomEvent::new(),
            entity_events: Vec::new(),
            entity_event: CustomEvent::new(),
            scene_events: SceneEvents::new(),
            // main_window moved to vetrace_editor crate
            scene_manager,
            core: EngineCore::new(),
            is_2d: is_2d,
            started_scripts: HashSet::new(),
            paused: false,
            saved_scene: None,
            ui_callbacks: Vec::new(),
            assets,
        };

        #[cfg(feature = "wgpu")]
        if is_2d {
            engine.renderer.set_post_fx_uniforms(PostFxUniforms {
                ..Default::default()
            });
        }

        engine.register_default_factories();
        engine.register_default_components();
        engine.ensure_generated_folder();
        engine.reload_scripts();
        engine.update_generated_components();
        engine.register_default_behaviours();

        {
            use egui::Color32;
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(Color32::WHITE);
            visuals.panel_fill = Color32::from_rgb(30, 20, 50);
            visuals.window_fill = Color32::from_rgb(25, 15, 40);
            engine.egui_ctx.set_visuals(visuals);
        }
        engine
    }
}