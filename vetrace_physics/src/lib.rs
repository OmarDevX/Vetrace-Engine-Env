//! Optional physics plugins for Vetrace.
//!
//! The default surface owns the Rapier 3D bridge. The `physics_2d` Cargo feature
//! adds an independent, renderer-neutral 2D rigid-body and collision plugin.
//! `vetrace_core` remains backend-agnostic.

pub mod actor_ext;
pub mod components;
pub mod scene_definitions;
#[cfg(feature = "physics_2d")]
pub mod physics_2d;

#[deprecated(note = "use scene_definitions")]
pub mod defs {
    pub use super::scene_definitions::*;
}
mod state;
mod cleanup;
mod colliders;
mod character;
mod sync;
mod raycast;
mod backend;
mod plugin;
#[cfg(feature = "gltf_collisions")]
mod gltf_collisions;

pub use actor_ext::{CharacterBodyBundle, PhysicsActorExt, RigidBodyBundle};
pub use backend::RapierPhysicsBackend;
pub use cleanup::{remove_physics_entity, teleport_body};
pub use components::*;
pub use scene_definitions::*;
pub use plugin::{add_dynamic_box, RapierPhysicsPlugin};
#[cfg(feature = "gltf_collisions")]
pub use gltf_collisions::{apply_gltf_imported_colliders, GltfCollisionApplyReport};
pub use raycast::raycast_colliders;
pub use state::PhysicsState;
#[cfg(feature = "physics_2d")]
pub use physics_2d::*;
