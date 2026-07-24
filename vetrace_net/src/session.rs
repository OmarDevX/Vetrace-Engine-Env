//! Reusable client/server session state.
//!
//! This module deliberately stores only generic network/session metadata. Game
//! data is supplied through the `G` type parameter and remains owned by games.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;

use vetrace_core::{Actor, Entity};

use crate::protocol::SequencedInput;
use crate::replication::InputHistory;
use crate::transport::TypedUdpChannel;

#[derive(Clone, Copy, Debug)]
pub struct SnapshotTimer {
    pub rate_hz: f32,
    pub accumulator: f32,
}

impl SnapshotTimer {
    pub fn new(rate_hz: f32) -> Self {
        Self { rate_hz: rate_hz.max(0.001), accumulator: 0.0 }
    }

    pub fn interval_seconds(&self) -> f32 {
        1.0 / self.rate_hz.max(0.001)
    }

    pub fn advance(&mut self, dt: f32) -> bool {
        self.accumulator += dt.max(0.0);
        if self.accumulator >= self.interval_seconds() {
            self.accumulator = 0.0;
            true
        } else {
            false
        }
    }

    pub fn advance_or_force(&mut self, dt: f32, force: bool) -> bool {
        self.accumulator += dt.max(0.0);
        if force || self.accumulator >= self.interval_seconds() {
            self.accumulator = 0.0;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}

impl Default for SnapshotTimer {
    fn default() -> Self {
        Self::new(30.0)
    }
}

#[derive(Clone, Debug)]
pub struct ClientInputStream<I> {
    next_seq: u64,
    pending: InputHistory<I>,
}

impl<I> ClientInputStream<I> {
    pub fn new(max_pending_inputs: usize) -> Self {
        Self { next_seq: 0, pending: InputHistory::new(max_pending_inputs) }
    }

    pub fn next_sequence(&mut self) -> u64 {
        self.next_seq = self.next_seq.saturating_add(1);
        self.next_seq
    }

    pub fn next_sequenced<T>(&mut self, client_id: Option<u64>, input: T) -> SequencedInput<T> {
        SequencedInput::new(client_id, self.next_sequence(), input)
    }

    pub fn push_pending(&mut self, seq: u64, input: I) {
        self.pending.push(seq, input);
    }

    pub fn ack_through(&mut self, seq: u64) {
        self.pending.ack_through(seq);
    }

    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    pub fn pending(&self) -> &InputHistory<I> {
        &self.pending
    }

    pub fn pending_mut(&mut self) -> &mut InputHistory<I> {
        &mut self.pending
    }

    pub fn last_acked(&self) -> u64 {
        self.pending.last_acked()
    }
}

impl<I> Default for ClientInputStream<I> {
    fn default() -> Self {
        Self::new(128)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NetEntityMap {
    by_net_id: HashMap<u64, Entity>,
}

impl NetEntityMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, net_id: u64) -> Option<Entity> {
        self.by_net_id.get(&net_id).copied()
    }

    pub fn insert(&mut self, net_id: u64, entity: Entity) -> Option<Entity> {
        self.by_net_id.insert(net_id, entity)
    }

    pub fn remove(&mut self, net_id: u64) -> Option<Entity> {
        self.by_net_id.remove(&net_id)
    }

    pub fn get_or_insert_with(&mut self, net_id: u64, spawn: impl FnOnce() -> Entity) -> Entity {
        *self.by_net_id.entry(net_id).or_insert_with(spawn)
    }
}

pub struct ServerSession<P, G> {
    pub endpoint: TypedUdpChannel<P>,
    pub clients: HashMap<SocketAddr, ServerClient<G>>,
    pub tick: u64,
    pub next_client_id: u64,
    pub snapshot_timer: SnapshotTimer,
}

impl<P, G> ServerSession<P, G> {
    pub fn new(endpoint: TypedUdpChannel<P>, first_client_id: u64, snapshot_rate_hz: f32) -> Self {
        Self {
            endpoint,
            clients: HashMap::new(),
            tick: 0,
            next_client_id: first_client_id,
            snapshot_timer: SnapshotTimer::new(snapshot_rate_hz),
        }
    }

    pub fn allocate_client_id(&mut self) -> u64 {
        let id = self.next_client_id;
        self.next_client_id = self.next_client_id.saturating_add(1);
        id
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    pub fn ensure_client_with(&mut self, addr: SocketAddr, create: impl FnOnce(u64) -> (Option<Entity>, G)) -> &ServerClient<G> {
        if !self.clients.contains_key(&addr) {
            let id = self.allocate_client_id();
            let (entity, game) = create(id);
            self.clients.insert(addr, ServerClient::new(id, addr, entity, game));
        }
        self.clients.get(&addr).expect("client was just inserted or already existed")
    }

    pub fn client_ack_seq(&self, addr: SocketAddr) -> u64 {
        self.clients.get(&addr).map(|client| client.last_input_seq).unwrap_or(0)
    }
}

pub struct ServerClient<G> {
    pub id: u64,
    pub addr: SocketAddr,
    pub entity: Option<Entity>,
    pub last_input_seq: u64,
    pub last_seen: Instant,
    pub game: G,
}

impl<G> ServerClient<G> {
    pub fn actor(&self) -> Option<Actor> { self.entity.map(Actor::from_entity) }

    pub fn new(id: u64, addr: SocketAddr, entity: Option<Entity>, game: G) -> Self {
        Self { id, addr, entity, last_input_seq: 0, last_seen: Instant::now(), game }
    }

    pub fn acknowledge_input(&mut self, seq: u64) {
        self.last_input_seq = self.last_input_seq.max(seq);
        self.last_seen = Instant::now();
    }
}

pub struct ClientSession<P, I> {
    pub endpoint: TypedUdpChannel<P>,
    pub server_addr: SocketAddr,
    pub client_id: Option<u64>,
    pub hello_timer: f32,
    pub input_stream: ClientInputStream<I>,
    pub entity_map: NetEntityMap,
}

impl<P, I> ClientSession<P, I> {
    pub fn new(endpoint: TypedUdpChannel<P>, server_addr: SocketAddr, max_pending_inputs: usize) -> Self {
        Self {
            endpoint,
            server_addr,
            client_id: None,
            hello_timer: 0.0,
            input_stream: ClientInputStream::new(max_pending_inputs),
            entity_map: NetEntityMap::new(),
        }
    }

    pub fn advance_hello_timer(&mut self, dt: f32, interval_seconds: f32) -> bool {
        self.hello_timer += dt.max(0.0);
        if self.client_id.is_none() && self.hello_timer >= interval_seconds.max(0.001) {
            self.hello_timer = 0.0;
            true
        } else {
            false
        }
    }

    pub fn accept_join(&mut self, client_id: u64) {
        self.client_id = Some(client_id);
        self.hello_timer = 0.0;
    }
}
