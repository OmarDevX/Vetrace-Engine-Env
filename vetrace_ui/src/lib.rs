//! Optional UI component/plugin crate for Vetrace.
//!
//! UI components live here instead of `vetrace_core` so games can skip UI
//! entirely or replace it with their own UI plugin.

pub mod components;

use std::any::Any;
use std::error::Error;

use vetrace_core::app::Plugin;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::Stage;

#[derive(Default)]
pub struct UiState {
    pub focused: Option<vetrace_core::Entity>,
    pub hovered: Option<vetrace_core::Entity>,
}

pub struct UiPlugin;

impl UiPlugin {
    pub fn new() -> Self { Self }
}

impl Default for UiPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for UiPlugin {
    fn name(&self) -> &'static str { "ui" }
    fn update_stage(&self) -> Stage { Stage::PostUpdate }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource(UiState::default());
        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_reflected_named::<components::UIScreenSpace>("vetrace.ui.screen_space", "UI Screen Space", "UI");
            cm.register_reflected_named::<components::UIWorldSpace>("vetrace.ui.world_space", "UI World Space", "UI");
            cm.register_reflected_named::<components::UILabel>("vetrace.ui.label", "UI Label", "UI");
            cm.register_reflected_named::<components::UIPanel>("vetrace.ui.panel", "UI Panel", "UI");
            cm.register_reflected_named::<components::UIButton>("vetrace.ui.button", "UI Button", "UI");
            cm.register_reflected_named::<components::UITextEditor>("vetrace.ui.text_editor", "UI Text Editor", "UI");
            cm.register_reflected_named::<components::UIList>("vetrace.ui.list", "UI List", "UI");
            cm.register_reflected_named::<components::UILayout>("vetrace.ui.layout", "UI Layout", "UI");
            cm.register_reflected_named::<components::ColorRect>("vetrace.ui.color_rect", "Color Rect", "UI");
            cm.register_reflected_named::<components::UIVisualStyle>("vetrace.ui.visual_style", "UI Visual Style", "UI");
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub use components::*;
