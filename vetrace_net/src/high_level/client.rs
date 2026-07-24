use std::net::SocketAddr;

use serde::de::DeserializeOwned;
use serde::Serialize;
use vetrace_core::{Actor, Entity};

use crate::protocol::{RpcRegistration, RpcRegistry, SequencedInput};
use crate::session::{ClientSession, NetEntityMap};
use crate::transport::TypedUdpChannel;

use super::{NetId, PeerId};

/// High-level client wrapper over the lower-level session.
pub struct MultiplayerClient<P, PendingInput> {
    pub session: ClientSession<P, PendingInput>,
    pub rpc_registry: RpcRegistry,
}

impl<P, PendingInput> MultiplayerClient<P, PendingInput> {
    pub fn new(endpoint: TypedUdpChannel<P>, server_addr: SocketAddr, max_pending_inputs: usize) -> Self {
        Self { session: ClientSession::new(endpoint, server_addr, max_pending_inputs), rpc_registry: RpcRegistry::new() }
    }

    pub fn client_id(&self) -> Option<PeerId> {
        self.session.client_id
    }

    pub fn server_addr(&self) -> SocketAddr {
        self.session.server_addr
    }

    pub fn accept_join(&mut self, client_id: PeerId) {
        self.session.accept_join(client_id);
    }

    pub fn ack_snapshot(&mut self, ack_seq: u64) {
        self.session.input_stream.ack_through(ack_seq);
    }

    pub fn push_prediction(&mut self, seq: u64, pending: PendingInput) {
        self.session.input_stream.push_pending(seq, pending);
    }

    pub fn clear_predictions(&mut self) {
        self.session.input_stream.clear_pending();
    }

    pub fn pending_predictions(&self) -> &crate::replication::InputHistory<PendingInput> {
        self.session.input_stream.pending()
    }

    pub fn mapped_actor(&self, net_id: NetId) -> Option<Actor> {
        self.session.entity_map.get(net_id).map(Actor::from_entity)
    }

    pub fn get_or_spawn_actor(&mut self, net_id: NetId, spawn: impl FnOnce() -> Actor) -> Actor {
        Actor::from_entity(
            self.session
                .entity_map
                .get_or_insert_with(net_id, || spawn().entity()),
        )
    }

    pub fn insert_actor_mapping(&mut self, net_id: NetId, actor: Actor) -> Option<Actor> {
        self.session
            .entity_map
            .insert(net_id, actor.entity())
            .map(Actor::from_entity)
    }

    #[deprecated(note = "use mapped_actor")]
    pub fn mapped_entity(&self, net_id: NetId) -> Option<Entity> {
        self.session.entity_map.get(net_id)
    }

    #[deprecated(note = "use get_or_spawn_actor")]
    pub fn get_or_spawn_entity(&mut self, net_id: NetId, spawn: impl FnOnce() -> Entity) -> Entity {
        self.session.entity_map.get_or_insert_with(net_id, spawn)
    }

    #[deprecated(note = "use insert_actor_mapping")]
    pub fn insert_entity_mapping(&mut self, net_id: NetId, entity: Entity) -> Option<Entity> {
        self.session.entity_map.insert(net_id, entity)
    }

    pub fn entity_map(&self) -> &NetEntityMap {
        &self.session.entity_map
    }

    pub fn entity_map_mut(&mut self) -> &mut NetEntityMap {
        &mut self.session.entity_map
    }

    /// Register a named RPC with Godot-like chainable configuration.
    pub fn register_rpc<Payload>(&mut self, name: impl Into<String>) -> RpcRegistration<'_, Payload> {
        self.rpc_registry.register_rpc::<Payload>(name)
    }
}

impl<P, PendingInput> MultiplayerClient<P, PendingInput>
where
    P: Serialize + DeserializeOwned,
{
    pub fn recv_packets(&mut self) -> Vec<(SocketAddr, P)> {
        self.session.endpoint.recv_many()
    }

    pub fn send_to_server(&self, packet: &P) -> std::io::Result<Option<usize>> {
        self.session.endpoint.send_to(self.session.server_addr, packet)
    }

    pub fn send_when_join_pending(
        &mut self,
        dt: f32,
        interval_seconds: f32,
        make_hello_packet: impl FnOnce() -> P,
    ) -> std::io::Result<Option<usize>> {
        if self.session.advance_hello_timer(dt, interval_seconds) {
            self.send_to_server(&make_hello_packet())
        } else {
            Ok(None)
        }
    }

    /// Send one input payload with an engine-owned sequence number.
    ///
    /// The payload remains game-owned. The game supplies the packet constructor
    /// because `vetrace_net` does not know the game's packet enum.
    pub fn send_input<InputPayload>(
        &mut self,
        input: InputPayload,
        make_packet: impl FnOnce(SequencedInput<InputPayload>) -> P,
    ) -> std::io::Result<(u64, Option<usize>)> {
        let seq = self.session.input_stream.next_sequence();
        let sequenced = SequencedInput::new(self.session.client_id, seq, input);
        let sent = self.send_to_server(&make_packet(sequenced))?;
        Ok((seq, sent))
    }
}
