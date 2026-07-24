use super::*;

#[derive(Clone, Debug)]
pub struct LanDiscoveryConfig {
    pub port: u16,
    pub protocol: u32,
    pub service_id: String,
    pub refresh_interval: std::time::Duration,
    pub server_expiry: std::time::Duration,
}

impl LanDiscoveryConfig {
    pub(super) fn shooter(port: u16) -> Self {
        Self {
            port,
            protocol: 1,
            service_id: "VETRACE_SHOOTER_DISCOVER".to_string(),
            refresh_interval: std::time::Duration::from_secs(2),
            server_expiry: std::time::Duration::from_secs(7),
        }
    }

    fn query(&self) -> Vec<u8> { format!("{}_V{}", self.service_id, self.protocol).into_bytes() }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ServerAdvertisement {
    protocol: u32,
    name: String,
    game_port: u16,
    players: u16,
    max_players: u16,
    map: String,
    in_lobby: bool,
}

#[derive(Clone, Debug)]
pub struct DiscoveredServer {
    pub name: String,
    pub addr: SocketAddr,
    pub players: u16,
    pub max_players: u16,
    pub map: String,
    pub in_lobby: bool,
    last_seen: std::time::Instant,
}

pub struct ServerBrowser {
    socket: std::net::UdpSocket,
    pub servers: Vec<DiscoveredServer>,
    last_refresh: std::time::Instant,
    config: LanDiscoveryConfig,
}

impl ServerBrowser {
    pub fn new(config: LanDiscoveryConfig) -> std::io::Result<Self> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        socket.set_broadcast(true)?;
        let initial_refresh = config.refresh_interval;
        Ok(Self { socket, servers: Vec::new(), last_refresh: std::time::Instant::now() - initial_refresh, config })
    }

    pub fn refresh(&mut self) {
        self.last_refresh = std::time::Instant::now();
        let query = self.config.query();
        let target = SocketAddr::from(([255, 255, 255, 255], self.config.port));
        let _ = self.socket.send_to(&query, target);
        // Loopback makes discovery reliable on hosts/VMs that suppress broadcast.
        let _ = self.socket.send_to(&query, SocketAddr::from(([127, 0, 0, 1], self.config.port)));
    }

    pub fn update(&mut self) {
        if self.last_refresh.elapsed() >= self.config.refresh_interval { self.refresh(); }
        let mut buffer = [0u8; 2048];
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((size, from)) => {
                    let Ok(ad) = serde_json::from_slice::<ServerAdvertisement>(&buffer[..size]) else { continue; };
                    if ad.protocol != self.config.protocol { continue; }
                    let addr = SocketAddr::new(from.ip(), ad.game_port);
                    if let Some(found) = self.servers.iter_mut().find(|server| server.addr == addr) {
                        found.name = ad.name;
                        found.players = ad.players;
                        found.max_players = ad.max_players;
                        found.map = ad.map;
                        found.in_lobby = ad.in_lobby;
                        found.last_seen = std::time::Instant::now();
                    } else {
                        self.servers.push(DiscoveredServer {
                            name: ad.name, addr, players: ad.players, max_players: ad.max_players,
                            map: ad.map, in_lobby: ad.in_lobby, last_seen: std::time::Instant::now(),
                        });
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        self.servers.retain(|server| server.last_seen.elapsed() < self.config.server_expiry);
        self.servers.sort_by(|a, b| a.name.cmp(&b.name).then(a.addr.cmp(&b.addr)));
    }
}

pub struct ServerAdvertiser {
    socket: std::net::UdpSocket,
    pub name: String,
    pub game_port: u16,
    config: LanDiscoveryConfig,
}

impl ServerAdvertiser {
    pub fn new(name: String, game_port: u16, config: LanDiscoveryConfig) -> std::io::Result<Self> {
        let socket = std::net::UdpSocket::bind(("0.0.0.0", config.port))?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, name, game_port, config })
    }

    pub fn update(&self, players: u16, max_players: u16, map: &str, in_lobby: bool) {
        let mut buffer = [0u8; 256];
        let query = self.config.query();
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((size, from)) if &buffer[..size] == query.as_slice() => {
                    let ad = ServerAdvertisement {
                        protocol: self.config.protocol, name: self.name.clone(), game_port: self.game_port,
                        players, max_players, map: map.to_string(), in_lobby,
                    };
                    if let Ok(bytes) = serde_json::to_vec(&ad) { let _ = self.socket.send_to(&bytes, from); }
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }
}
