//! Vetrace Engine - A modern, modular game engine
//!
//! This crate provides a clean, production-ready game engine with:
//! - ECS (Entity Component System) architecture
//! - Modern rendering pipeline
//! - Asset management system
//! - Plugin architecture
//! - Clean separation between engine and editor

// Main engine
pub mod engine;
pub use engine::{
    apply_component_data, export_component_data, Actor, Engine, EngineCore, PhysicsState, Prefab,
    Stage, World,
};

// Core systems
pub mod rendering;
pub use rendering::{Renderer, RenderParams};

// ECS
pub mod ecs;
pub use ecs::{Entity, Component};

// Asset management
pub mod assets;
pub use assets::AssetManager;

// Input system
pub mod input;
pub use input::Input;

// Other core modules
pub mod math;
pub mod materials;
pub mod custom_material;
pub mod gpu;
pub mod lod;
pub use lod::{
    AutoLod, AutoLodProcessor, LodLevel, LodSettings, LodStats, MeshData, SimplifiedMesh,
};
pub use custom_material::{CustomMaterial, MaterialParameter, RaytraceShaderCompiler};
// Legacy modules (for migration)
pub mod scene {
    pub mod bvh;
    pub mod factories;
    pub mod loader;
    pub mod object;
    pub mod scene;
    pub mod tri_bvh;
}

pub mod events;
pub mod shared;
pub mod net;
pub mod systems;
pub mod ui;
pub mod behaviour;
pub mod components;
pub mod inspector;
pub mod app;

// Legacy exports
pub use events::Event;
pub use input::{window::WindowManager};
pub use ecs::behaviour::Behaviour;
