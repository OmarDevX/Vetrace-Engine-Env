use std::net::SocketAddr;

use super::packets::NetPacket;
use super::transport::NetSocket;

/// Simple UDP client that can connect and send [`NetPacket::Ping`].
pub struct NetClient {
    pub socket: NetSocket,
    pub server_addr: SocketAddr,
}

impl NetClient {
    /// Connect to a server at `server_addr` using a local port chosen by the OS.
    pub fn connect(server_addr: SocketAddr) -> std::io::Result<Self> {
        let local = if server_addr.is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        Ok(Self {
            socket: NetSocket::bind(local)?,
            server_addr,
        })
    }

    /// Send a ping packet to the server.
    pub fn send_ping(&mut self) {
        self.socket
            .send_queue
            .push_back((self.server_addr, NetPacket::Ping));
        let _ = self.socket.flush_send_queue();
    }

    /// Poll incoming packets from the server.
    pub fn poll(&mut self) {
        let _ = self.socket.poll_recv_queue();
    }
}

impl Drop for NetClient {
    fn drop(&mut self) {
        self.socket
            .send_queue
            .push_back((self.server_addr, NetPacket::Disconnect));
        let _ = self.socket.flush_send_queue();
    }
}
