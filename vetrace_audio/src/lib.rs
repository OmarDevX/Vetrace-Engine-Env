//! Optional audio component/plugin crate for Vetrace.
//!
//! The crate owns generic audio ECS components plus an optional Kira backend.
//! Games spawn normal `AudioSource` entities and optionally an `AudioListener`;
//! no game-specific audio behavior belongs in `vetrace_core`.

pub mod components;

use std::any::Any;
use std::error::Error;

use vetrace_core::app::Plugin;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::Stage;

#[derive(Clone, Debug)]
pub struct AudioState {
    pub enabled: bool,
    pub backend: &'static str,
}

impl Default for AudioState {
    fn default() -> Self {
        Self { enabled: false, backend: "none" }
    }
}

pub struct AudioPlugin {
    backend: backend::AudioBackend,
}

impl AudioPlugin {
    pub fn new() -> Self { Self { backend: backend::AudioBackend::new() } }
}

impl Default for AudioPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for AudioPlugin {
    fn name(&self) -> &'static str { "audio" }
    fn update_stage(&self) -> Stage { Stage::PostUpdate }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        self.backend.initialize();
        engine.insert_resource(AudioState {
            enabled: self.backend.enabled(),
            backend: self.backend.name(),
        });
        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_named::<components::AudioSource>("vetrace.audio.source", "Audio Source");
            cm.register_named::<components::AudioListener>("vetrace.audio.listener", "Audio Listener");
        }
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        self.backend.update(engine);
        if let Some(state) = engine.get_resource_mut::<AudioState>() {
            state.enabled = self.backend.enabled();
            state.backend = self.backend.name();
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub use components::*;

mod backend;
