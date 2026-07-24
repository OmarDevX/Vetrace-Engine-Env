//! Engine-owned multiplayer protocol and game-loop drivers.
//!
//! Games provide only their input, RPC, replicated state/event, handshake, and
//! message payloads. The driver owns the common wire envelope, compatibility
//! validation, input sequencing, RPC delivery, join retry, and snapshot/event
//! plumbing.

mod client;
mod compatibility;
mod protocol;
mod server;

pub use client::GameClientDriver;
pub use compatibility::{CompatibilityManifest, CompatibilityMismatch};
pub use protocol::{
    ClientGameEvent, ClientTimeout, GameNetPacket, GamePacket, ReplicatedEventQueue,
    ServerGameEvent,
};
pub use server::GameServerDriver;

pub const DEFAULT_HELLO_INTERVAL_SECONDS: f32 = 0.5;
pub const DEFAULT_RPC_RESEND_INTERVAL_SECONDS: f32 = 0.15;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_reports_named_gameplay_mismatch() {
        let host = CompatibilityManifest::new(3).with_gameplay_hash("gameplay_bundle", 10);
        let client = CompatibilityManifest::new(3).with_gameplay_hash("gameplay_bundle", 11);
        assert!(matches!(host.mismatch(&client), Some(CompatibilityMismatch::Gameplay { name, .. }) if name == "gameplay_bundle"));
    }

    #[test]
    fn replicated_events_drain_once() {
        let mut events = ReplicatedEventQueue::default();
        events.extend([1, 2]);
        assert_eq!(events.drain(), vec![1, 2]);
        assert!(events.is_empty());
    }
}
