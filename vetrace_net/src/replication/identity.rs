use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationAuthority {
    Server,
    Client { owner_id: u64 },
}

impl Default for ReplicationAuthority {
    fn default() -> Self { Self::Server }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct NetworkIdentity {
    pub net_id: u64,
    pub owner_id: Option<u64>,
    pub authority: ReplicationAuthority,
}
