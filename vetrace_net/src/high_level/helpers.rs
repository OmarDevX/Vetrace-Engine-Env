use std::time::Instant;

use crate::replication::NetworkIdentity;

use super::PeerId;

/// Small authority helper for gameplay code.
pub fn is_owned_by(identity: &NetworkIdentity, peer_id: PeerId) -> bool {
    identity.owner_id == Some(peer_id)
}

/// Small liveness helper useful for timeout policies in games.
pub fn seconds_since_last_seen(last_seen: Instant) -> f32 {
    last_seen.elapsed().as_secs_f32()
}
