//! High-level multiplayer facade.
//!
//! The low-level pieces (`TypedUdpChannel`, `ServerSession`, `ClientSession`,
//! `SnapshotFrame`, `RpcCall`, etc.) remain public. This module adds a smaller,
//! Godot-like surface for games that only want to declare networked entities,
//! send inputs/RPCs, and consume snapshot frames without hand-writing the same
//! boilerplate in every example.
//!
//! This layer is intentionally component-agnostic. It does not know what a
//! `Transform`, `Health`, `DoorState`, or `Inventory` is. Games register those
//! via `ReplicatedComponentAdapter` implementations and attach their chosen
//! replicated component configs to entities.

mod client;
mod entity_builder;
mod helpers;
mod rpc;
mod server;

/// Stable peer id used by high-level multiplayer helpers.
pub type PeerId = u64;

/// Stable network entity id.
pub type NetId = u64;

/// Read-only view of a replicated state item.
///
/// Unlike the earlier temporary transform-specific version, this trait only
/// requires stable network identity/tick metadata. The actual replicated payload
/// can be any game or component snapshot.
pub trait ReplicatedSnapshotState {
    fn net_id(&self) -> NetId;
    fn tick(&self) -> u64;
}

pub use client::MultiplayerClient;
pub use entity_builder::{network_actor, network_entity, NetworkActorBuilder, NetworkEntityBuilder};
pub use helpers::{is_owned_by, seconds_since_last_seen};
pub use rpc::MultiplayerRpc;
pub use server::MultiplayerServer;
