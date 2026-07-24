//! Optional networking plugin for Vetrace.
//!
//! UDP/server-client details live here, not in `vetrace_core`.

pub mod high_level;
pub mod game_driver;
pub mod protocol;
pub mod replication;
pub mod session;
pub mod transport;

pub use high_level::{is_owned_by, network_actor, network_entity, seconds_since_last_seen, MultiplayerClient, MultiplayerRpc, MultiplayerServer, NetId, NetworkActorBuilder, NetworkEntityBuilder, PeerId, ReplicatedSnapshotState};
pub use game_driver::{ClientGameEvent, ClientTimeout, CompatibilityManifest, CompatibilityMismatch, GameClientDriver, GameNetPacket, GamePacket, GameServerDriver, ReplicatedEventQueue, ServerGameEvent, DEFAULT_HELLO_INTERVAL_SECONDS, DEFAULT_RPC_RESEND_INTERVAL_SECONDS};
pub use protocol::{NetDelivery, NetSequence, PendingReliableRpc, RpcAck, RpcCall, RpcConfig, RpcDeliveryState, RpcInbox, RpcMode, RpcOutbox, RpcReceiveResult, RpcRegistration, RpcRegistry, RpcRejectReason, RpcSync, RpcTarget, SequencedInput, SnapshotFrame, TransferMode};
pub use replication::{
    ComponentInterpolationState, ComponentSnapshotRef, GenericComponentInterpolator, InputHistory,
    InterpolationStep, NetworkIdentity, ReplicatedComponentAdapter,
    ReplicatedComponentConfig, ReplicatedComponentList, ReplicatedComponentSnapshot,
    ReplicationAuthority, lerp_angle,
};
pub use session::{ClientInputStream, ClientSession, NetEntityMap, ServerClient, ServerSession, SnapshotTimer};
pub use transport::{decode_packet, encode_packet, TypedUdpChannel, DEFAULT_MAX_PACKET_SIZE};

use std::any::Any;
use std::collections::VecDeque;
use std::error::Error;
use std::net::{SocketAddr, UdpSocket};

use vetrace_core::app::Plugin;
use vetrace_core::backends::NetBackend;
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::Stage;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NetPacket {
    Ping,
    Pong,
    Custom { kind: String, data: Vec<u8> },
}

#[derive(Default, Debug)]
pub struct InputBuffer {
    pub packets: VecDeque<NetPacket>,
}

#[derive(Default, Debug)]
pub struct UnreliableSync;


/// Small reusable UDP transport wrapper for game protocols.
///
/// The packet format stays game-owned: `vetrace_net` only owns socket setup,
/// nonblocking receive loops, peer tracking, and send helpers. This lets games
/// validate/use the networking crate without forcing their protocol into core.
pub struct UdpChannel {
    socket: UdpSocket,
    peers: Vec<SocketAddr>,
}

impl UdpChannel {
    pub fn bind(addr: impl AsRef<str>) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(addr.as_ref())?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, peers: Vec::new() })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    pub fn add_peer(&mut self, addr: SocketAddr) {
        if !self.peers.contains(&addr) {
            self.peers.push(addr);
        }
    }

    pub fn remove_peer(&mut self, addr: SocketAddr) -> bool {
        let previous_len = self.peers.len();
        self.peers.retain(|peer| *peer != addr);
        self.peers.len() != previous_len
    }

    pub fn set_broadcast(&self, enabled: bool) -> std::io::Result<()> {
        self.socket.set_broadcast(enabled)
    }

    pub fn peers(&self) -> &[SocketAddr] {
        &self.peers
    }

    pub fn send_to(&self, addr: SocketAddr, bytes: &[u8]) -> std::io::Result<usize> {
        self.socket.send_to(bytes, addr)
    }

    pub fn broadcast(&self, bytes: &[u8]) {
        for peer in &self.peers {
            let _ = self.socket.send_to(bytes, peer);
        }
    }

    pub fn recv_many(&mut self, max_packet_size: usize) -> Vec<(SocketAddr, Vec<u8>)> {
        self.recv_up_to(max_packet_size, usize::MAX)
    }

    pub fn recv_up_to(
        &mut self,
        max_packet_size: usize,
        max_packets: usize,
    ) -> Vec<(SocketAddr, Vec<u8>)> {
        let mut out = Vec::new();
        let mut buf = vec![0_u8; max_packet_size.max(1)];
        while out.len() < max_packets {
            match self.socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    self.add_peer(addr);
                    out.push((addr, buf[..len].to_vec()));
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    eprintln!("vetrace_net udp error: {err}");
                    break;
                }
            }
        }
        out
    }
}

impl std::fmt::Debug for UdpChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpChannel")
            .field("local_addr", &self.socket.local_addr().ok())
            .field("peers", &self.peers)
            .finish()
    }
}

pub struct UdpNetResource {
    pub socket: Option<UdpSocket>,
    pub peers: Vec<SocketAddr>,
}

impl Default for UdpNetResource {
    fn default() -> Self {
        Self { socket: None, peers: Vec::new() }
    }
}

#[derive(Default)]
pub struct UdpNetBackend;

impl UdpNetBackend {
    pub fn new() -> Self { Self }
}

impl NetBackend for UdpNetBackend {
    fn poll(&mut self, engine: &mut Engine) {
        let Some(net) = engine.get_resource_mut::<UdpNetResource>() else { return; };
        let Some(socket) = &net.socket else { return; };
        let mut buf = [0_u8; 2048];
        loop {
            match socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if !net.peers.contains(&addr) {
                        net.peers.push(addr);
                    }
                    if let Ok(packet) = bincode::deserialize::<NetPacket>(&buf[..len]) {
                        println!("net packet from {addr}: {packet:?}");
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    eprintln!("net poll error: {err}");
                    break;
                }
            }
        }
    }

    fn send_state(&mut self, engine: &Engine) {
        let Some(net) = engine.get_resource::<UdpNetResource>() else { return; };
        let Some(socket) = &net.socket else { return; };
        let Ok(bytes) = bincode::serialize(&NetPacket::Ping) else { return; };
        for peer in &net.peers {
            let _ = socket.send_to(&bytes, peer);
        }
    }
}

pub struct NetPlugin;

impl NetPlugin {
    pub fn new() -> Self { Self }
}

impl Default for NetPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for NetPlugin {
    fn name(&self) -> &'static str { "net" }
    fn update_stage(&self) -> Stage { Stage::PreUpdate }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        engine.insert_resource::<Box<dyn NetBackend>>(Box::new(UdpNetBackend::new()));
        engine.insert_resource(UdpNetResource::default());
        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register_named::<InputBuffer>("vetrace.net.input_buffer", "Input Buffer");
            cm.register_named::<UnreliableSync>("vetrace.net.unreliable_sync", "Unreliable Sync");
            cm.register_named::<NetworkIdentity>("vetrace.net.network_identity", "Network Identity");
            cm.register_named::<ReplicatedComponentList>("vetrace.net.replicated_components", "Replicated Components");
        }
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        let _ = engine.with_resource_removed::<Box<dyn NetBackend>, _>(
            |backend, engine| backend.poll(engine),
        );
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

#[cfg(test)]
mod udp_channel_tests {
    use super::*;

    #[test]
    fn udp_peer_tracking_deduplicates_and_removes() {
        let mut channel = UdpChannel::bind("127.0.0.1:0").unwrap();
        let peer: SocketAddr = "127.0.0.1:3456".parse().unwrap();
        channel.add_peer(peer);
        channel.add_peer(peer);
        assert_eq!(channel.peers(), &[peer]);
        assert!(channel.remove_peer(peer));
        assert!(!channel.remove_peer(peer));
        assert!(channel.peers().is_empty());
    }
}
