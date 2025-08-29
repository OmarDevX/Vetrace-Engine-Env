use crate::rendering::Renderer;
use crate::scene::scene::Scene;
#[cfg(feature = "use_epi")]
use crate::rendering::EguiRenderer;
use egui::{Context as EguiContext, Event};

/// Manages all rendering-related functionality
pub struct RenderingManager {
    pub renderer: Renderer,
    pub scene: Scene,
    pub egui_ctx: EguiContext,
    #[cfg(feature = "use_epi")]
    pub egui_renderer: EguiRenderer,
    pub egui_events: Vec<Event>,
}

impl RenderingManager {
    pub fn new(
        renderer: Renderer,
        scene: Scene,
        egui_ctx: EguiContext,
        #[cfg(feature = "use_epi")]
        egui_renderer: EguiRenderer,
    ) -> Self {
        Self {
            renderer,
            scene,
            egui_ctx,
            #[cfg(feature = "use_epi")]
            egui_renderer,
            egui_events: Vec::new(),
        }
    }

    /// Add an egui event to the queue
    pub fn add_egui_event(&mut self, event: Event) {
        self.egui_events.push(event);
    }

    /// Clear egui events
    pub fn clear_egui_events(&mut self) {
        self.egui_events.clear();
    }

    /// Get the renderer reference
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Get mutable renderer reference
    pub fn renderer_mut(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// Get the scene reference
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Get mutable scene reference
    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// Get the egui context
    pub fn egui_context(&self) -> &EguiContext {
        &self.egui_ctx
    }

    /// Get mutable egui context
    pub fn egui_context_mut(&mut self) -> &mut EguiContext {
        &mut self.egui_ctx
    }
}
