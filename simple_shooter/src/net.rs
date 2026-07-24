use glam::Vec3;
use serde::{Deserialize, Serialize};
use vetrace_net::{GameNetPacket, ReplicatedComponentSnapshot, ReplicatedSnapshotState, SnapshotFrame};

use crate::components::{MatchPhase, SessionRules, ShooterInput};
use crate::replication::TransformSnapshot;

pub const SHOOTER_PROTOCOL_VERSION: u32 = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShooterHello {
    pub name: String,
    pub color_seed: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShooterWelcome {
    pub mod_settings: ShooterModSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShooterMessage {
    AdminCommand(ShooterAdminCommand),
    MapRequest { revision: u64, first_missing_chunk: u32 },
    ModSettings { mod_fingerprint: u64, settings: ShooterModSettings },
    Text { text: String },
    Shutdown { reason: String },
    MapManifest(MapManifest),
    MapChunk { revision: u64, chunk_index: u32, bytes: Vec<u8> },
    Session { phase: MatchPhase, admin_id: u64, rules: SessionRules },
}

pub type ShooterPacket = GameNetPacket<
    NetInput,
    ShooterRpc,
    PlayerSnapshot,
    ShotSnapshot,
    ShooterMessage,
    ShooterHello,
    ShooterWelcome,
>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapManifest {
    pub map_index: u8,
    pub name: String,
    pub revision: u64,
    pub total_bytes: u64,
    pub chunk_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShooterAdminCommand {
    StartGame,
    StopGame,
    PreviousMap,
    NextMap,
    PreviousMod,
    NextMod,
    ToggleMod,
    ToggleBots,
    BotCountDown,
    BotCountUp,
    MaxPlayersDown,
    MaxPlayersUp,
    DifficultyDown,
    DifficultyUp,
    SpeedDown,
    SpeedUp,
    GravityDown,
    GravityUp,
    JumpDown,
    JumpUp,
    KillLimitDown,
    KillLimitUp,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShooterModSettings {
    pub movement_multiplier: f32,
    pub jump_multiplier: f32,
    pub gravity_scale: f32,
    pub vignette_strength: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct NetInput {
    pub move_x: f32,
    pub move_y: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fire: bool,
    pub jump: bool,
}

impl From<ShooterInput> for NetInput {
    fn from(input: ShooterInput) -> Self {
        Self {
            move_x: input.movement.x,
            move_y: input.movement.y,
            yaw: input.yaw,
            pitch: input.pitch,
            fire: input.fire,
            jump: input.jump,
        }
    }
}

impl From<NetInput> for ShooterInput {
    fn from(input: NetInput) -> Self {
        Self {
            movement: glam::Vec2::new(input.move_x, input.move_y),
            yaw: input.yaw,
            pitch: input.pitch,
            fire: input.fire,
            jump: input.jump,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ShooterRpc {
    FireWeapon { yaw: f32, pitch: f32 },
}

pub type ServerSnapshot = SnapshotFrame<PlayerSnapshot, ShotSnapshot>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub transform: ReplicatedComponentSnapshot<TransformSnapshot>,
    pub velocity: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub weapon_id: String,
    pub name: String,
    pub health: i32,
    pub alive: bool,
    pub color_seed: u64,
    pub kills: u32,
    pub deaths: u32,
    pub last_killer_id: Option<u64>,
    pub last_killer_name: String,
    pub last_kill_damage: i32,
}

impl PlayerSnapshot {
    pub fn id(&self) -> u64 { self.transform.net_id }
    pub fn tick(&self) -> u64 { self.transform.tick }
    pub fn position_vec3(&self) -> Vec3 { self.transform.data.translation_vec3() }
    pub fn velocity_vec3(&self) -> Vec3 { Vec3::new(self.velocity[0], self.velocity[1], self.velocity[2]) }
    pub fn rotation_quat(&self) -> glam::Quat { self.transform.data.rotation_quat() }
    pub fn yaw(&self) -> f32 { self.yaw }
    pub fn pitch(&self) -> f32 { self.pitch }
}

impl ReplicatedSnapshotState for PlayerSnapshot {
    fn net_id(&self) -> u64 { self.id() }
    fn tick(&self) -> u64 { self.transform.tick }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShotSnapshot {
    pub weapon_id: String,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub shooter_id: u64,
    pub hit_id: Option<u64>,
}

pub fn v3(value: Vec3) -> [f32; 3] {
    [value.x, value.y, value.z]
}
