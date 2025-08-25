use crate::scene::object::Object;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Basic network packet used for client/server communication.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum NetPacket {
    /// Simple connection ping used for keepalive/handshake.
    Ping,
    /// Response to a ping containing timing information.
    Pong,
    /// Client is disconnecting gracefully.
    Disconnect,
    /// Client is requesting to connect and may send optional info.
    Connect(ClientInfo),
    /// Server assigns the player's entity id.
    AssignEntity(u32),
    /// Acknowledge receipt of a reliable packet.
    Ack(u16),
    /// Wrapper for reliable delivery containing sequence id.
    Reliable { seq: u16, packet: Box<NetPacket> },
    /// Spawn a new object with the given entity id and initial data.
    SpawnObject { entity: u32, object: Object },
    /// Despawn an object on all clients.
    DespawnObject { entity: u32 },
    /// Input data from a client for a specific simulation tick.
    Input { tick: u32, input: InputData },
    /// Snapshot of world state for a given tick.
    Snapshot {
        tick: u32,
        entities: Vec<EntitySnapshot>,
    },
    /// Remote procedure call invocation.
    Rpc {
        entity: u32,
        method: String,
        args: Vec<Value>,
    },
    /// Updated component data for an entity.
    ComponentUpdate {
        entity: u32,
        component: String,
        data: Vec<u8>,
    },
    /// Batch update for multiple components of an entity.
    ComponentBatch {
        entity: u32,
        updates: Vec<(String, Vec<u8>)>,
    },
    /// Unreliable transform update for networked entities.
    TransformSync {
        entity: u32,
        position: [f32; 3],
        orientation: [f32; 4],
        velocity: [f32; 3],
    },
    /// Game-specific payload that the engine does not interpret.
    Custom { kind: String, data: Vec<u8> },
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ClientInfo {
    pub name: String,
}

/// Placeholder for user input data transmitted each tick.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct InputData {
    /// Raw bytes describing input state. Format defined by the game.
    pub bytes: Vec<u8>,
}

/// Snapshot of a single entity's state containing raw component data.
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct EntitySnapshot {
    /// The entity identifier this snapshot belongs to.
    pub entity: u32,
    /// Serialized components for this entity `(component_name, bytes)`.
    pub components: Vec<(String, Vec<u8>)>,
}
