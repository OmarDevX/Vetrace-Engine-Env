use std::marker::PhantomData;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::time::Instant;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{MultiplayerClient, MultiplayerRpc, RpcRegistration, RpcTarget, TypedUdpChannel};

use super::{
    ClientGameEvent, ClientTimeout, CompatibilityManifest, GameNetPacket,
    DEFAULT_HELLO_INTERVAL_SECONDS, DEFAULT_RPC_RESEND_INTERVAL_SECONDS,
};

type Packet<I, R, S, E, M, H, W> = GameNetPacket<I, R, S, E, M, H, W>;

pub struct GameClientDriver<I, R, S, E, M, H, W, Pending> {
    pub multiplayer: MultiplayerClient<Packet<I, R, S, E, M, H, W>, Pending>,
    pub rpc: MultiplayerRpc<R>,
    compatibility: CompatibilityManifest,
    hello: H,
    hello_interval: f32,
    rpc_resend_interval: f32,
    connected_at: Instant,
    last_server_packet_at: Instant,
    _wire: PhantomData<(I, S, E, M, W)>,
}

impl<I, R, S, E, M, H, W, Pending> GameClientDriver<I, R, S, E, M, H, W, Pending> {
    pub fn new(
        endpoint: TypedUdpChannel<Packet<I, R, S, E, M, H, W>>,
        server_addr: SocketAddr,
        max_pending_inputs: usize,
        compatibility: CompatibilityManifest,
        hello: H,
    ) -> Self {
        Self {
            multiplayer: MultiplayerClient::new(endpoint, server_addr, max_pending_inputs),
            rpc: MultiplayerRpc::new(1),
            compatibility,
            hello,
            hello_interval: DEFAULT_HELLO_INTERVAL_SECONDS,
            rpc_resend_interval: DEFAULT_RPC_RESEND_INTERVAL_SECONDS,
            connected_at: Instant::now(),
            last_server_packet_at: Instant::now(),
            _wire: PhantomData,
        }
    }

    pub fn set_compatibility(&mut self, compatibility: CompatibilityManifest) { self.compatibility = compatibility; }
    pub fn set_hello(&mut self, hello: H) { self.hello = hello; }
    pub fn set_hello_interval(&mut self, seconds: f32) { self.hello_interval = seconds.max(0.001); }
    pub fn set_rpc_resend_interval(&mut self, seconds: f32) { self.rpc_resend_interval = seconds.max(0.001); }

    pub fn timeout(&self, join_seconds: f32, server_loss_seconds: f32) -> Option<ClientTimeout> {
        if self.multiplayer.client_id().is_some() {
            (self.last_server_packet_at.elapsed().as_secs_f32() > server_loss_seconds)
                .then_some(ClientTimeout::ServerLost)
        } else {
            (self.connected_at.elapsed().as_secs_f32() > join_seconds).then_some(ClientTimeout::Join)
        }
    }

    pub fn register_rpc<T>(&mut self, name: impl Into<String>) -> RpcRegistration<'_, T> {
        self.rpc.register_rpc::<T>(name)
    }

    pub fn rpc_named(&mut self, name: impl Into<String>, target: RpcTarget, payload: R) -> u64
    where R: Clone {
        self.rpc.rpc_named(name, self.multiplayer.client_id(), target, payload)
    }

    pub fn leave(&self)
    where
        I: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned,
        S: Serialize + DeserializeOwned, E: Serialize + DeserializeOwned,
        M: Serialize + DeserializeOwned, H: Serialize + DeserializeOwned,
        W: Serialize + DeserializeOwned,
    {
        let _ = self.multiplayer.send_to_server(&GameNetPacket::Leave);
    }

    pub fn send_message(&self, message: M)
    where
        I: Serialize + DeserializeOwned, R: Serialize + DeserializeOwned,
        S: Serialize + DeserializeOwned, E: Serialize + DeserializeOwned,
        M: Serialize + DeserializeOwned, H: Serialize + DeserializeOwned,
        W: Serialize + DeserializeOwned,
    {
        let _ = self.multiplayer.send_to_server(&GameNetPacket::Message(message));
    }
}

impl<I, R, S, E, M, H, W, Pending> GameClientDriver<I, R, S, E, M, H, W, Pending>
where
    I: Clone + Serialize + DeserializeOwned,
    R: Clone + Serialize + DeserializeOwned,
    S: Clone + Serialize + DeserializeOwned,
    E: Clone + Serialize + DeserializeOwned,
    M: Clone + Serialize + DeserializeOwned,
    H: Clone + Serialize + DeserializeOwned,
    W: Clone + Serialize + DeserializeOwned,
{
    pub fn send_input(&mut self, input: I) -> std::io::Result<(u64, Option<usize>)> {
        self.multiplayer.send_input(input, GameNetPacket::Input)
    }

    pub fn poll(&mut self, dt: f32) -> Vec<ClientGameEvent<S, E, M, W>> {
        let compatibility = self.compatibility.clone();
        let hello = self.hello.clone();
        let _ = self.multiplayer.send_when_join_pending(dt, self.hello_interval, || {
            GameNetPacket::Hello { compatibility, payload: hello }
        });
        self.flush_rpcs(dt);
        let mut events = Vec::new();
        let packets = self.multiplayer.recv_packets();
        if !packets.is_empty() { self.last_server_packet_at = Instant::now(); }
        for (_addr, packet) in packets {
            match packet {
                GameNetPacket::Welcome { client_id, tick, compatibility, payload } => {
                    if let Some(mismatch) = self.compatibility.mismatch(&compatibility) {
                        events.push(ClientGameEvent::Rejected(mismatch.to_string()));
                    } else {
                        self.multiplayer.accept_join(client_id);
                        events.push(ClientGameEvent::Joined { client_id, tick, payload });
                    }
                }
                GameNetPacket::Snapshot(snapshot) => {
                    self.multiplayer.ack_snapshot(snapshot.ack_seq);
                    events.push(ClientGameEvent::Snapshot(snapshot));
                }
                GameNetPacket::Rpc(call) => {
                    let result = self.rpc.receive_rpc_call(self.multiplayer.server_addr(), call, None);
                    if let Some(ack) = result.ack {
                        let _ = self.multiplayer.send_to_server(&GameNetPacket::RpcAck(ack));
                    }
                }
                GameNetPacket::RpcAck(ack) => { self.rpc.acknowledge(ack); }
                GameNetPacket::Message(message) => events.push(ClientGameEvent::Message(message)),
                GameNetPacket::Rejected { reason } => events.push(ClientGameEvent::Rejected(reason)),
                _ => {}
            }
        }
        events
    }

    fn flush_rpcs(&mut self, dt: f32) {
        let mut calls = self.rpc.drain_outgoing().collect::<Vec<_>>();
        calls.extend(self.rpc.reliable_resends_due(dt, self.rpc_resend_interval));
        for call in calls {
            let _ = self.multiplayer.send_to_server(&GameNetPacket::Rpc(call));
        }
    }
}

impl<I, R, S, E, M, H, W, Pending> Deref for GameClientDriver<I, R, S, E, M, H, W, Pending> {
    type Target = MultiplayerClient<Packet<I, R, S, E, M, H, W>, Pending>;
    fn deref(&self) -> &Self::Target { &self.multiplayer }
}

impl<I, R, S, E, M, H, W, Pending> DerefMut for GameClientDriver<I, R, S, E, M, H, W, Pending> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.multiplayer }
}
