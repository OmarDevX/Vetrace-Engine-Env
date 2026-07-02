use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::ecs::Entity;

use super::packets::NetPacket;
use super::transport::NetSocket;

pub type ClientId = u32;

/// Basic UDP server that accepts clients sending [`NetPacket::Ping`].
pub struct NetServer {
    pub socket: NetSocket,
    pub clients: HashMap<SocketAddr, ClientId>,
    pub entity_map: HashMap<ClientId, Entity>,
    pub disconnected: Vec<Entity>,
    last_heard: HashMap<SocketAddr, Instant>,
    next_client_id: ClientId,
}

impl NetServer {
    /// Create a new server listening on the given address.
    pub fn new(addr: SocketAddr) -> std::io::Result<Self> {
        Ok(Self {
            socket: NetSocket::bind(addr)?,
            clients: HashMap::new(),
            entity_map: HashMap::new(),
            disconnected: Vec::new(),
            last_heard: HashMap::new(),
            next_client_id: 1,
        })
    }

    /// Poll incoming packets and register new clients.
    pub fn poll(&mut self) {
        let _ = self.socket.poll_recv_queue();
        self.check_timeouts();
        let mut remaining = VecDeque::new();
        while let Some((addr, packet)) = self.socket.recv_queue.pop_front() {
            match packet {
                NetPacket::Ping => {
                    // Register new client if needed
                    let id = self.clients.entry(addr).or_insert_with(|| {
                        let id = self.next_client_id;
                        self.next_client_id += 1;
                        id
                    });
                    // Echo back ping as acknowledgement
                    self.socket
                        .send_queue
                        .push_back((addr, NetPacket::Ping));
                    // Ensure entity mapping exists
                    self.entity_map.entry(*id).or_insert(Entity(0));
                    self.last_heard.insert(addr, Instant::now());
                }
                NetPacket::Disconnect => {
                    if let Some(id) = self.clients.remove(&addr) {
                        if let Some(entity) = self.entity_map.remove(&id) {
                            self.disconnected.push(entity);
                        }
                    }
                    self.last_heard.remove(&addr);
                }
                other => {
                    self.last_heard.insert(addr, Instant::now());
                    remaining.push_back((addr, other))
                }
            }
        }
        self.socket.recv_queue = remaining;
        let _ = self.socket.flush_send_queue();
    }

    fn check_timeouts(&mut self) {
        const TIMEOUT: Duration = Duration::from_secs(5);
        let now = Instant::now();
        let mut expired = Vec::new();
        for (&addr, &last) in &self.last_heard {
            if now.duration_since(last) > TIMEOUT {
                expired.push(addr);
            }
        }
        for addr in expired {
            self.last_heard.remove(&addr);
            if let Some(id) = self.clients.remove(&addr) {
                if let Some(entity) = self.entity_map.remove(&id) {
                    self.disconnected.push(entity);
                }
            }
        }
    }
}
