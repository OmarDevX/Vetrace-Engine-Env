use std::net::SocketAddr;

use serde::de::DeserializeOwned;
use serde::Serialize;
use vetrace_core::{Actor, Entity};

use crate::protocol::{RpcRegistration, RpcRegistry, SnapshotFrame};
use crate::session::{ServerClient, ServerSession};
use crate::transport::TypedUdpChannel;

use super::PeerId;

/// High-level server wrapper over the lower-level session.
pub struct MultiplayerServer<P, G> {
    pub session: ServerSession<P, G>,
    pub rpc_registry: RpcRegistry,
}

impl<P, G> MultiplayerServer<P, G> {
    pub fn new(endpoint: TypedUdpChannel<P>, first_client_id: PeerId, snapshot_rate_hz: f32) -> Self {
        Self { session: ServerSession::new(endpoint, first_client_id, snapshot_rate_hz), rpc_registry: RpcRegistry::new() }
    }

    pub fn tick(&self) -> u64 {
        self.session.tick
    }

    pub fn advance_tick(&mut self) {
        self.session.advance_tick();
    }

    pub fn should_send_snapshot(&mut self, dt: f32, force: bool) -> bool {
        self.session.snapshot_timer.advance_or_force(dt, force)
    }

    pub fn clients(&self) -> impl Iterator<Item = &ServerClient<G>> {
        self.session.clients.values()
    }

    pub fn clients_mut(&mut self) -> impl Iterator<Item = &mut ServerClient<G>> {
        self.session.clients.values_mut()
    }

    pub fn ensure_client_actor_with(
        &mut self,
        addr: SocketAddr,
        create: impl FnOnce(PeerId) -> (Option<Actor>, G),
    ) -> &ServerClient<G> {
        self.session.ensure_client_with(addr, |id| {
            let (actor, game) = create(id);
            (actor.map(Actor::entity), game)
        })
    }

    #[deprecated(note = "use ensure_client_actor_with")]
    pub fn ensure_client_with(
        &mut self,
        addr: SocketAddr,
        create: impl FnOnce(PeerId) -> (Option<Entity>, G),
    ) -> &ServerClient<G> {
        self.session.ensure_client_with(addr, create)
    }

    pub fn client_ack_seq(&self, addr: SocketAddr) -> u64 {
        self.session.client_ack_seq(addr)
    }

    pub fn client_mut(&mut self, addr: SocketAddr) -> Option<&mut ServerClient<G>> {
        self.session.clients.get_mut(&addr)
    }

    /// Register a named RPC with Godot-like chainable configuration.
    pub fn register_rpc<Payload>(&mut self, name: impl Into<String>) -> RpcRegistration<'_, Payload> {
        self.rpc_registry.register_rpc::<Payload>(name)
    }
}

impl<P, G> MultiplayerServer<P, G>
where
    P: Serialize + DeserializeOwned,
{
    pub fn recv_packets(&mut self) -> Vec<(SocketAddr, P)> {
        self.session.endpoint.recv_many()
    }

    pub fn send_to(&self, addr: SocketAddr, packet: &P) -> std::io::Result<Option<usize>> {
        self.session.endpoint.send_to(addr, packet)
    }

    pub fn broadcast(&self, packet: &P) {
        self.session.endpoint.broadcast(packet);
    }

    /// Send one snapshot frame to every connected client, using that client's
    /// current input acknowledgement sequence.
    pub fn send_snapshot_frames<State, Event>(
        &self,
        states: Vec<State>,
        events: Vec<Event>,
        make_packet: impl Fn(SnapshotFrame<State, Event>) -> P,
    ) where
        State: Clone,
        Event: Clone,
    {
        for addr in self.session.clients.keys().copied().collect::<Vec<_>>() {
            let frame = SnapshotFrame::new(self.session.tick, self.client_ack_seq(addr))
                .with_states(states.clone())
                .with_events(events.clone());
            let packet = make_packet(frame);
            let _ = self.send_to(addr, &packet);
        }
    }
}
