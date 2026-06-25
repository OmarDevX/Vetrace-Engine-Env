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
    Actor, Engine, EngineCore, PhysicsState, Prefab, Stage, World, apply_component_data,
    export_component_data,
};

// Core systems
pub mod rendering;
pub use rendering::{RenderParams, Renderer};

// ECS
pub mod ecs;
pub use ecs::{Component, Entity};

// Asset management
pub mod assets;
pub use assets::AssetManager;

// Input system
pub mod input;
pub use input::Input;

// Other core modules
pub mod custom_material;
pub mod gpu;
pub mod lod;
pub mod materials;
pub mod math;
pub use custom_material::{
    CustomMaterial, MaterialOutputContract, MaterialParameter, RaytraceShaderCompiler,
};
pub use lod::{
    AutoLod, AutoLodProcessor, LodLevel, LodSettings, LodStats, MeshData, SimplifiedMesh,
};
// Legacy modules (for migration)
pub mod scene {
    pub mod bvh;
    pub mod factories;
    pub mod loader;
    pub mod object;
    pub mod scene;
    pub mod tri_bvh;
}

pub mod app;
pub mod behaviour;
pub mod components;
pub mod events;
pub mod inspector;
pub mod net;
pub mod shared;
pub mod systems;
pub mod ui;

// Legacy exports
pub use ecs::behaviour::Behaviour;
pub use events::Event;
pub use input::window::WindowManager;
