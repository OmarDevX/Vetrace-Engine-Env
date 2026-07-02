//! High level engine API and supporting utilities.
//!
//! The [`engine`] module exposes the [`Engine`] type which drives window
//! creation, rendering and ECS management. `core` contains low level ECS helper
//! functions and structures while `component_io` provides helpers for
//! serializing component data.

mod access;
mod actor;
pub mod component_io;
mod components;
pub mod core;
pub mod engine;
pub mod engine_v2;
pub mod engine_builder;
pub mod managers;
mod init;
mod objects;
pub mod physics;
mod prefab;
mod run;
mod scripts;
mod stage;
mod scene_manager;
pub mod ui;
mod world;
pub use actor::Actor;
pub use prefab::Prefab;
pub use scene_manager::SceneManager;
pub use stage::Stage;
pub use world::World;

pub use component_io::{apply_component_data, export_component_data};
pub use core::EngineCore;
pub use engine::Engine;
pub use engine_v2::Engine as EngineV2;
pub use engine_builder::{EngineBuilder, create_engine, create_2d_engine};
pub use managers::{
    RenderingManager, InputManager, ScriptingManager,
    ComponentManager, EventManager, UIManager
};
pub use physics::PhysicsState;
