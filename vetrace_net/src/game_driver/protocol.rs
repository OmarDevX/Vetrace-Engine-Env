use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::{PeerId, RpcAck, RpcCall, SequencedInput, SnapshotFrame};

use super::CompatibilityManifest;

/// Standard engine wire envelope. `Message` is the escape hatch for game-only
/// traffic such as chat, lobby commands, or streamed map chunks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameNetPacket<Input, Rpc, State, Event, Message, Hello, Welcome> {
    Hello { compatibility: CompatibilityManifest, payload: Hello },
    Input(SequencedInput<Input>),
    Rpc(RpcCall<Rpc>),
    RpcAck(RpcAck),
    Leave,
    Welcome {
        client_id: PeerId,
        tick: u64,
        compatibility: CompatibilityManifest,
        payload: Welcome,
    },
    Snapshot(SnapshotFrame<State, Event>),
    Message(Message),
    Rejected { reason: String },
}

pub type GamePacket<I, R, S, E, M, H, W> = GameNetPacket<I, R, S, E, M, H, W>;

#[derive(Clone, Debug)]
pub enum ServerGameEvent<Input, Message, Hello> {
    JoinRequested { addr: SocketAddr, payload: Hello },
    Input { addr: SocketAddr, client_id: PeerId, input: Input },
    Message { addr: SocketAddr, client_id: PeerId, message: Message },
    DisconnectRequested { addr: SocketAddr },
}

#[derive(Clone, Debug)]
pub enum ClientGameEvent<State, Event, Message, Welcome> {
    Joined { client_id: PeerId, tick: u64, payload: Welcome },
    Snapshot(SnapshotFrame<State, Event>),
    Message(Message),
    Rejected(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientTimeout {
    Join,
    ServerLost,
}

#[derive(Clone, Debug)]
pub struct ReplicatedEventQueue<E> {
    events: Vec<E>,
}

impl<E> Default for ReplicatedEventQueue<E> {
    fn default() -> Self { Self { events: Vec::new() } }
}

impl<E> ReplicatedEventQueue<E> {
    pub fn push(&mut self, event: E) { self.events.push(event); }
    pub fn extend(&mut self, events: impl IntoIterator<Item = E>) { self.events.extend(events); }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn drain(&mut self) -> Vec<E> { std::mem::take(&mut self.events) }
}
