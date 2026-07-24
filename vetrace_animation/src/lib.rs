//! Optional animation component/plugin crate for Vetrace.

pub mod components;
pub mod runtime;

use std::any::Any;
use std::error::Error;

use vetrace_core::app::Plugin;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::Stage;

#[derive(Default)]
pub struct AnimationState;

pub struct AnimationPlugin;

impl AnimationPlugin {
    pub fn new() -> Self { Self }
}

impl Default for AnimationPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for AnimationPlugin {
    fn name(&self) -> &'static str { "animation" }
    fn update_stage(&self) -> Stage { Stage::PostUpdate }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource(AnimationState::default());
        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_named::<components::Lerp>("vetrace.animation.lerp", "Lerp");
            cm.register_named::<components::LerpData>("vetrace.animation.lerp_data", "Lerp Data");
            cm.register_named::<components::Animation>("vetrace.animation.animation", "Animation");
            cm.register_named::<components::AnimationPlayer>("vetrace.animation.player", "Animation Player");
            cm.register_named::<components::MorphTargets>("vetrace.animation.morph_targets", "Morph Targets");
            cm.register_named::<components::MorphWeights>("vetrace.animation.morph_weights", "Morph Weights");
            cm.register_named::<components::Skin>("vetrace.animation.skin", "Skin");
        }
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        runtime::update_animation_clocks(engine, dt);
        runtime::update_animation_players(engine, dt);
        runtime::update_lerps(engine, dt);
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

pub use components::*;
