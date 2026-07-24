//! Vetrace Core - lightweight framework kernel.
//!
//! `vetrace_core` intentionally contains only the generic framework surface:
//! app/plugin dispatch, the core `Engine`, ECS/world storage, scenes, resource
//! storage, and cross-plugin backend traits. Feature implementations such as
//! Rapier physics, Lua scripting, UDP networking, rendering, audio, editor UI,
//! and runtime apps live in separate crates that depend on this crate.

extern crate self as vetrace_core;

pub mod actor;
pub mod app;
pub mod backends;
pub mod schedule;
pub mod query;
pub mod reflection;
pub mod events;
pub mod hierarchy;
pub mod commands;
pub mod bundle;
pub mod components;
pub mod ecs;
pub mod engine;
pub mod input;
pub mod resources;
pub mod scene;
pub mod systems;

pub use actor::{Actor, ActorBuilder, ActorDestroyed, ActorError};
pub use app::{App, AppBuilder, AppRunner, Plugin, PluginManager};
pub use backends::{NetBackend, PhysicsBackend, ProfilerBackend, RaycastHit, RenderBackend, ScriptingBackend};
pub use components::builtins::{ActorId, Children, GlobalTransform, Metadata, Name, ObjectRef, Parent, Timer, Transform, TransformDirty};
pub use bundle::Bundle;
pub use commands::{Commands, SpawnCommandBuilder};
pub use ecs::{Component, Entity, World};
pub use events::{EventReader, EventWriter, Events};
pub use hierarchy::Hierarchy;
pub use query::{MutQuery, MutQueryWith, Query, QuerySpec};
pub use reflection::{
    ComponentSchema, DynamicValue, FieldKind, FieldPath, FieldSchema, FieldSegment,
    NumericRange, ReflectionError, VetraceComponent, VetraceEnum, merge_dynamic_patch,
};
pub use schedule::{FixedTime, Schedule, Stage, SystemFn};
pub use engine::{ComponentDescriptor, ComponentManager, Engine};
pub use input::InputState;
pub use resources::{DebugTextOverlayPanel, Resources};
pub use scene::{EntityDef, SceneDef};
pub use systems::{HierarchyPlugin, TimerPlugin, propagate_global_transforms, tick_timers};
pub use vetrace_engine_macros::{VetraceComponent, VetraceEnum};

/// Small behaviour hook kept for compatibility with older Vetrace examples.
/// New code should generally prefer `Plugin::update` plus typed resources.
pub trait Behaviour: 'static {
    fn update(&mut self, _engine: &mut Engine, _dt: f32) {}
}

#[cfg(test)]
mod architecture_tests;
