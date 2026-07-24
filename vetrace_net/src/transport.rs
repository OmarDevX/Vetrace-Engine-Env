//! Typed packet transport helpers.
//!
//! Games own their packet enums. `vetrace_net` owns the reusable mechanics:
//! serialize/deserialize, UDP send/receive loops, peer tracking, and broadcast.

use std::marker::PhantomData;
use std::net::SocketAddr;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::UdpChannel;

pub const DEFAULT_MAX_PACKET_SIZE: usize = 4096;

pub fn encode_packet<P: Serialize>(packet: &P) -> Option<Vec<u8>> {
    bincode::serialize(packet).ok()
}

pub fn decode_packet<P: DeserializeOwned>(bytes: &[u8]) -> Option<P> {
    bincode::deserialize(bytes).ok()
}

/// A UDP channel bound to one game-owned packet type.
///
/// The packet type remains game-side; this wrapper only removes duplicated
/// bincode and UDP receive-loop code from each game.
pub struct TypedUdpChannel<P> {
    channel: UdpChannel,
    max_packet_size: usize,
    _marker: PhantomData<fn() -> P>,
}

impl<P> TypedUdpChannel<P> {
    pub fn from_channel(channel: UdpChannel) -> Self {
        Self { channel, max_packet_size: DEFAULT_MAX_PACKET_SIZE, _marker: PhantomData }
    }

    pub fn with_max_packet_size(mut self, max_packet_size: usize) -> Self {
        self.max_packet_size = max_packet_size.max(1);
        self
    }

    pub fn bind(addr: impl AsRef<str>) -> std::io::Result<Self> {
        Ok(Self::from_channel(UdpChannel::bind(addr)?))
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.channel.local_addr()
    }

    pub fn add_peer(&mut self, addr: SocketAddr) {
        self.channel.add_peer(addr);
    }

    pub fn peers(&self) -> &[SocketAddr] {
        self.channel.peers()
    }

    pub fn raw(&self) -> &UdpChannel {
        &self.channel
    }

    pub fn raw_mut(&mut self) -> &mut UdpChannel {
        &mut self.channel
    }
}

impl<P> TypedUdpChannel<P>
where
    P: Serialize + DeserializeOwned,
{
    pub fn send_to(&self, addr: SocketAddr, packet: &P) -> std::io::Result<Option<usize>> {
        let Some(bytes) = encode_packet(packet) else {
            return Ok(None);
        };
        self.channel.send_to(addr, &bytes).map(Some)
    }

    pub fn broadcast(&self, packet: &P) {
        if let Some(bytes) = encode_packet(packet) {
            self.channel.broadcast(&bytes);
        }
    }

    pub fn recv_many(&mut self) -> Vec<(SocketAddr, P)> {
        self.channel
            .recv_many(self.max_packet_size)
            .into_iter()
            .filter_map(|(addr, bytes)| decode_packet::<P>(&bytes).map(|packet| (addr, packet)))
            .collect()
    }
}

impl<P> std::fmt::Debug for TypedUdpChannel<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedUdpChannel")
            .field("channel", &self.channel)
            .field("max_packet_size", &self.max_packet_size)
            .finish()
    }
}
