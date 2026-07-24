//! Generic replication primitives.
//!
//! This module deliberately does **not** know about engine/game components such
//! as `Transform`, `Health`, `Inventory`, `DoorState`, or `VehicleState`.
//! It only provides reusable identity, ownership, input-history, replicated
//! component registration, and generic interpolation storage. Games or engine
//! integration crates register the concrete component adapters they want to
//! synchronize.

pub mod component;
pub mod history;
pub mod identity;
pub mod interpolation;

pub use component::{
    ComponentSnapshotRef, ReplicatedComponentAdapter, ReplicatedComponentConfig,
    ReplicatedComponentList, ReplicatedComponentSnapshot,
};
pub use history::InputHistory;
pub use identity::{NetworkIdentity, ReplicationAuthority};
pub use interpolation::{ComponentInterpolationState, GenericComponentInterpolator, InterpolationStep};

pub fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut delta = (b - a) % std::f32::consts::TAU;
    if delta > std::f32::consts::PI { delta -= std::f32::consts::TAU; }
    if delta < -std::f32::consts::PI { delta += std::f32::consts::TAU; }
    a + delta * t
}
