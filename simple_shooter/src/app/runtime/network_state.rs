use super::*;

pub struct ServerState {
    pub net: ShooterServerNet,
    /// Set during App::update, consumed during App::render after physics.
    /// This keeps snapshots authoritative to the final physics Transform,
    /// including physical/visual tilt, without making vetrace_net know about
    /// Transform or Simple Shooter.
    pub snapshot_due_after_physics: bool,
    pub last_mod_settings: Option<ShooterModSettings>,
    pub last_session: Option<(u8, u64, SessionRules, Vec<ScoreStanding>)>,
    pub mod_settings_resend_elapsed: f32,
    pub session_resend_elapsed: f32,
    pub max_players: u16,
    pub transport_player_present: bool,
}

impl ServerState {
    pub fn new(endpoint: TypedUdpChannel<ShooterPacket>, max_players: u16) -> Self {
        let mut net = GameServerDriver::new(
            endpoint,
            FIRST_REMOTE_PLAYER_ID,
            SERVER_SNAPSHOT_RATE_HZ,
            CompatibilityManifest::new(SHOOTER_PROTOCOL_VERSION),
        );
        net.register_rpc::<ShooterRpc>("fire_weapon")
            .any_peer()
            .call_remote()
            .unreliable_ordered();
        Self {
            net,
            snapshot_due_after_physics: false,
            last_mod_settings: None,
            last_session: None,
            mod_settings_resend_elapsed: 0.0,
            session_resend_elapsed: 0.0,
            max_players: max_players.max(1),
            transport_player_present: true,
        }
    }
}

pub struct ShooterClientData {
    pub name: String,
    pub color_seed: u64,
    pub last_input: ShooterInput,
    pub pending_fire: Option<(f32, f32)>,
}

#[derive(Clone, Copy, Debug)]
pub struct PredictedInput {
    pub input: ShooterInput,
    pub dt: f32,
}

pub struct ClientState {
    pub net: ShooterClientNet,
    pub transform_interpolation: TransformInterpolator,
    pub name: String,
    pub color_seed: u64,
    pub map_transfer: Option<ClientMapTransfer>,
    pub pending_hosted_session: Option<PendingHostedSession>,
    pub hosted_map_revisions: std::collections::HashMap<u8, u64>,
}

impl ClientState {
    pub fn new(endpoint: TypedUdpChannel<ShooterPacket>, server_addr: SocketAddr, name: String, color_seed: u64) -> Self {
        let mut net = GameClientDriver::new(
            endpoint,
            server_addr,
            128,
            CompatibilityManifest::new(SHOOTER_PROTOCOL_VERSION),
            ShooterHello { name: name.clone(), color_seed },
        );
        net.register_rpc::<ShooterRpc>("fire_weapon")
            .any_peer()
            .call_remote()
            .unreliable_ordered();
        Self {
            net,
            transform_interpolation: TransformInterpolator::new(),
            name,
            color_seed,
            map_transfer: None,
            pending_hosted_session: None,
            hosted_map_revisions: std::collections::HashMap::new(),
        }
    }
}
