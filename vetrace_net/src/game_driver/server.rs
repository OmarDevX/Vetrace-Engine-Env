use std::marker::PhantomData;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;
use serde::Serialize;
use vetrace_core::Actor;

use crate::{
    MultiplayerRpc, MultiplayerServer, PeerId, RpcCall, RpcRegistration, RpcTarget,
    ServerClient, TypedUdpChannel,
};

use super::{
    CompatibilityManifest, GameNetPacket, ReplicatedEventQueue, ServerGameEvent,
    DEFAULT_RPC_RESEND_INTERVAL_SECONDS,
};

type Packet<I, R, S, E, M, H, W> = GameNetPacket<I, R, S, E, M, H, W>;

/// Server-side reusable game driver.
pub struct GameServerDriver<I, R, S, E, M, H, W, G> {
    pub multiplayer: MultiplayerServer<Packet<I, R, S, E, M, H, W>, G>,
    pub rpc: MultiplayerRpc<R>,
    compatibility: CompatibilityManifest,
    replicated_events: ReplicatedEventQueue<E>,
    rpc_resend_interval: f32,
    _state: PhantomData<S>,
}

impl<I, R, S, E, M, H, W, G> GameServerDriver<I, R, S, E, M, H, W, G> {
    pub fn new(
        endpoint: TypedUdpChannel<Packet<I, R, S, E, M, H, W>>,
        first_client_id: PeerId,
        snapshot_rate_hz: f32,
        compatibility: CompatibilityManifest,
    ) -> Self {
        Self {
            multiplayer: MultiplayerServer::new(endpoint, first_client_id, snapshot_rate_hz),
            rpc: MultiplayerRpc::new(1),
            compatibility,
            replicated_events: ReplicatedEventQueue::default(),
            rpc_resend_interval: DEFAULT_RPC_RESEND_INTERVAL_SECONDS,
            _state: PhantomData,
        }
    }

    pub fn set_compatibility(&mut self, compatibility: CompatibilityManifest) {
        self.compatibility = compatibility;
    }

    pub fn compatibility(&self) -> &CompatibilityManifest { &self.compatibility }

    pub fn set_rpc_resend_interval(&mut self, seconds: f32) {
        self.rpc_resend_interval = seconds.max(0.001);
    }

    pub fn register_rpc<T>(&mut self, name: impl Into<String>) -> RpcRegistration<'_, T> {
        self.rpc.register_rpc::<T>(name)
    }

    pub fn queue_event(&mut self, event: E) { self.replicated_events.push(event); }
    pub fn queue_events(&mut self, events: impl IntoIterator<Item = E>) { self.replicated_events.extend(events); }
    pub fn has_pending_events(&self) -> bool { !self.replicated_events.is_empty() }

    pub fn timed_out_clients(&self, timeout_seconds: f32) -> Vec<SocketAddr> {
        self.multiplayer
            .clients()
            .filter(|client| client.last_seen.elapsed().as_secs_f32() > timeout_seconds)
            .map(|client| client.addr)
            .collect()
    }

    pub fn is_connected(&self, addr: SocketAddr) -> bool {
        self.multiplayer.session.clients.contains_key(&addr)
    }

    pub fn remove_client(&mut self, addr: SocketAddr) -> Option<ServerClient<G>> {
        self.multiplayer.session.clients.remove(&addr)
    }

    pub fn accept_with(
        &mut self,
        addr: SocketAddr,
        create: impl FnOnce(PeerId) -> (Option<Actor>, G),
        welcome: W,
    ) -> PeerId
    where
        I: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned,
        S: Serialize + DeserializeOwned, E: Serialize + DeserializeOwned,
        M: Serialize + DeserializeOwned, H: Serialize + DeserializeOwned,
        W: Serialize + DeserializeOwned,
    {
        let client_id = self.multiplayer.ensure_client_actor_with(addr, create).id;
        let packet = GameNetPacket::Welcome {
            client_id,
            tick: self.multiplayer.tick(),
            compatibility: self.compatibility.clone(),
            payload: welcome,
        };
        let _ = self.multiplayer.send_to(addr, &packet);
        client_id
    }

    pub fn reject(&self, addr: SocketAddr, reason: impl Into<String>)
    where
        I: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned,
        S: Serialize + DeserializeOwned, E: Serialize + DeserializeOwned,
        M: Serialize + DeserializeOwned, H: Serialize + DeserializeOwned,
        W: Serialize + DeserializeOwned,
    {
        let _ = self.multiplayer.send_to(addr, &GameNetPacket::Rejected { reason: reason.into() });
    }

    pub fn send_message(&self, addr: SocketAddr, message: M)
    where
        I: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned,
        S: Serialize + DeserializeOwned, E: Serialize + DeserializeOwned,
        M: Serialize + DeserializeOwned, H: Serialize + DeserializeOwned,
        W: Serialize + DeserializeOwned,
    {
        let _ = self.multiplayer.send_to(addr, &GameNetPacket::Message(message));
    }

    pub fn broadcast_message(&self, message: M)
    where
        I: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned,
        S: Serialize + DeserializeOwned, E: Serialize + DeserializeOwned,
        M: Serialize + DeserializeOwned + Clone, H: Serialize + DeserializeOwned,
        W: Serialize + DeserializeOwned,
    {
        self.multiplayer.broadcast(&GameNetPacket::Message(message));
    }

    pub fn drain_rpcs(&mut self) -> impl Iterator<Item = RpcCall<R>> + '_ {
        self.rpc.drain_incoming()
    }

    pub fn rpc_named(
        &mut self,
        name: impl Into<String>,
        target: RpcTarget,
        payload: R,
    ) -> u64
    where
        R: Clone,
    {
        self.rpc.rpc_named(name, None, target, payload)
    }
}

impl<I, R, S, E, M, H, W, G> GameServerDriver<I, R, S, E, M, H, W, G>
where
    I: Clone + Serialize + DeserializeOwned,
    R: Clone + Serialize + DeserializeOwned,
    S: Clone + Serialize + DeserializeOwned,
    E: Clone + Serialize + DeserializeOwned,
    M: Clone + Serialize + DeserializeOwned,
    H: Clone + Serialize + DeserializeOwned,
    W: Clone + Serialize + DeserializeOwned,
{
    pub fn poll(&mut self, dt: f32) -> Vec<ServerGameEvent<I, M, H>> {
        self.flush_rpcs(dt);
        let mut events = Vec::new();
        for (addr, packet) in self.multiplayer.recv_packets() {
            if let Some(client) = self.multiplayer.client_mut(addr) {
                client.last_seen = std::time::Instant::now();
            }
            match packet {
                GameNetPacket::Hello { compatibility, payload } => {
                    if let Some(mismatch) = self.compatibility.mismatch(&compatibility) {
                        self.reject(addr, mismatch.to_string());
                    } else {
                        events.push(ServerGameEvent::JoinRequested { addr, payload });
                    }
                }
                GameNetPacket::Input(sequenced) => {
                    if let Some(client) = self.multiplayer.client_mut(addr) {
                        if sequenced.client_id.is_none() || sequenced.client_id == Some(client.id) {
                            client.acknowledge_input(sequenced.seq);
                            events.push(ServerGameEvent::Input {
                                addr,
                                client_id: client.id,
                                input: sequenced.input,
                            });
                        }
                    }
                }
                GameNetPacket::Rpc(mut call) => {
                    if let Some(client_id) = self.multiplayer.client_mut(addr).map(|client| client.id) {
                        call.from_client_id = Some(client_id);
                        let result = self.rpc.receive_rpc_call(addr, call, Some(client_id));
                        if let Some(ack) = result.ack {
                            let _ = self.multiplayer.send_to(addr, &GameNetPacket::RpcAck(ack));
                        }
                    }
                }
                GameNetPacket::RpcAck(ack) => { self.rpc.acknowledge(ack); }
                GameNetPacket::Message(message) => {
                    if let Some(client_id) = self.multiplayer.client_mut(addr).map(|client| client.id) {
                        events.push(ServerGameEvent::Message { addr, client_id, message });
                    }
                }
                GameNetPacket::Leave => events.push(ServerGameEvent::DisconnectRequested { addr }),
                _ => {}
            }
        }
        events
    }

    fn flush_rpcs(&mut self, dt: f32) {
        let mut calls = self.rpc.drain_outgoing().collect::<Vec<_>>();
        calls.extend(self.rpc.reliable_resends_due(dt, self.rpc_resend_interval));
        for call in calls {
            let recipients: Vec<SocketAddr> = match call.target.clone() {
                RpcTarget::Client(id) => self.multiplayer.clients().filter(|client| client.id == id).map(|client| client.addr).collect(),
                RpcTarget::AllClients => self.multiplayer.clients().map(|client| client.addr).collect(),
                RpcTarget::AllExcept(id) => self.multiplayer.clients().filter(|client| client.id != id).map(|client| client.addr).collect(),
                _ => Vec::new(),
            };
            for addr in recipients {
                let _ = self.multiplayer.send_to(addr, &GameNetPacket::Rpc(call.clone()));
            }
        }
    }

    pub fn flush_snapshot(&mut self, states: Vec<S>) {
        let events = self.replicated_events.drain();
        self.multiplayer.send_snapshot_frames(states, events, GameNetPacket::Snapshot);
    }
}

impl<I, R, S, E, M, H, W, G> Deref for GameServerDriver<I, R, S, E, M, H, W, G> {
    type Target = MultiplayerServer<Packet<I, R, S, E, M, H, W>, G>;
    fn deref(&self) -> &Self::Target { &self.multiplayer }
}

impl<I, R, S, E, M, H, W, G> DerefMut for GameServerDriver<I, R, S, E, M, H, W, G> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.multiplayer }
}
