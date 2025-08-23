//! Networking primitives for Vetrace Engine.

mod client;
pub mod packets;
pub mod rpc;
mod server;
pub mod sync;
pub mod tick;
pub mod transport;

pub use client::NetClient;
pub use packets::{ClientInfo, EntitySnapshot, InputData, NetPacket};
pub use server::{ClientId, NetServer};
pub use sync::{
    NetSyncComponent, NetSyncHooks, NetSyncRegistry, apply_snapshots, collect_snapshots,
    register_sync_component, register_sync_component_with_filter,
};
pub use tick::{TICK_DURATION, TICK_RATE, TickManager};
pub use transport::{NetRole, NetSocket};
