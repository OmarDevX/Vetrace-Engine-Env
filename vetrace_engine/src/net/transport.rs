use std::collections::VecDeque;
use std::io::{self, ErrorKind};
use std::net::{SocketAddr, UdpSocket};

use super::packets::NetPacket;

/// Role of the network node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetRole {
    /// Client connecting to a remote server.
    Client,
    /// Server accepting incoming clients.
    Server,
    /// Networking disabled.
    Offline,
}

/// Thin wrapper around [`UdpSocket`] providing send/receive queues.
pub struct NetSocket {
    socket: UdpSocket,
    pub recv_queue: VecDeque<(SocketAddr, NetPacket)>,
    pub send_queue: VecDeque<(SocketAddr, NetPacket)>,
    pub reliable_queue: VecDeque<(SocketAddr, (u16, NetPacket))>,
    next_seq: u16,
}

impl NetSocket {
    /// Bind a new UDP socket to `addr`.
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            recv_queue: VecDeque::new(),
            send_queue: VecDeque::new(),
            reliable_queue: VecDeque::new(),
            next_seq: 0,
        })
    }

    /// Send all queued packets.
    pub fn flush_send_queue(&mut self) -> io::Result<()> {
        while let Some((addr, packet)) = self.send_queue.pop_front() {
            let data =
                bincode::serialize(&packet).map_err(|e| io::Error::new(ErrorKind::Other, e))?;
            let _ = self.socket.send_to(&data, addr);
        }
        for &(addr, (seq, ref packet)) in &self.reliable_queue {
            let wrapper = NetPacket::Reliable {
                seq,
                packet: Box::new(packet.clone()),
            };
            let data =
                bincode::serialize(&wrapper).map_err(|e| io::Error::new(ErrorKind::Other, e))?;
            let _ = self.socket.send_to(&data, addr);
        }
        Ok(())
    }

    /// Queue a packet to be sent unreliably.
    pub fn send(&mut self, addr: SocketAddr, packet: NetPacket) {
        self.send_queue.push_back((addr, packet));
    }

    /// Queue a reliable packet. Returns sequence id used for ack tracking.
    pub fn send_reliable(&mut self, addr: SocketAddr, packet: NetPacket) -> u16 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        self.reliable_queue.push_back((addr, (seq, packet)));
        seq
    }

    /// Remove acknowledged packets from the reliable queue.
    pub fn acknowledge(&mut self, seq: u16) {
        self.reliable_queue
            .retain(|&(_, (s, _))| s != seq);
    }

    /// Receive any pending packets and push them to the recv queue.
    pub fn poll_recv_queue(&mut self) -> io::Result<()> {
        loop {
            let mut buf = [0u8; 1400];
            match self.socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if let Ok(packet) = bincode::deserialize::<NetPacket>(&buf[..len]) {
                        match packet {
                            NetPacket::Ack(seq) => {
                                self.acknowledge(seq);
                            }
                            NetPacket::Reliable { seq, packet } => {
                                self.send(addr, NetPacket::Ack(seq));
                                self.recv_queue.push_back((addr, *packet));
                            }
                            other => {
                                self.recv_queue.push_back((addr, other));
                            }
                        }
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}
