use glam::Vec3;
use serde::{Deserialize, Serialize};

pub const MAX_HEALTH: i32 = 100;
pub const PLAYER_RADIUS: f32 = 0.45;
pub const PLAYER_HEIGHT: f32 = 1.8;
/// Visual clearance above the ground for the rendered body. The physics capsule
/// still uses PLAYER_HEIGHT, but the visible cuboid is shortened so the WGPU
/// inverted-hull outline does not sink into the arena floor.
pub const PLAYER_VISUAL_HOVER: f32 = 0.12;
pub const PLAYER_VISUAL_HEIGHT: f32 = PLAYER_HEIGHT - PLAYER_VISUAL_HOVER * 2.0;
pub const MOVE_SPEED: f32 = 4.2;
pub const JUMP_SPEED: f32 = 4.8;
pub const FPS_MOUSE_SENSITIVITY: f32 = 0.0018;
/// Eye height measured from the player's feet/base, matching normal FPS
/// convention. The ECS Transform is the capsule/body center, so gameplay code
/// must use FPS_EYE_LOCAL_Y when adding to Transform.translation.
pub const FPS_EYE_HEIGHT: f32 = 1.55;
pub const FPS_EYE_LOCAL_Y: f32 = FPS_EYE_HEIGHT - PLAYER_HEIGHT * 0.5;
pub const GROUND_Y: f32 = -0.02;
pub const RESPAWN_DELAY: f32 = 5.0;
pub const SHOOT_RANGE: f32 = 60.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShooterPlayer {
    pub id: u64,
    pub name: String,
    pub health: i32,
    pub alive: bool,
    pub respawn_timer: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub kills: u32,
    pub deaths: u32,
    pub last_killer_id: Option<u64>,
    pub last_killer_name: String,
    pub last_kill_damage: i32,
    pub life_damage_by_attacker: Vec<(u64, i32)>,
}

impl ShooterPlayer {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            health: MAX_HEALTH,
            alive: true,
            respawn_timer: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            kills: 0,
            deaths: 0,
            last_killer_id: None,
            last_killer_name: String::new(),
            last_kill_damage: 0,
            life_damage_by_attacker: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FirstPersonController {
    pub yaw: f32,
    pub pitch: f32,
    pub mouse_sensitivity: f32,
    pub grounded: bool,
}

impl Default for FirstPersonController {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            mouse_sensitivity: FPS_MOUSE_SENSITIVITY,
            grounded: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FreeFlightController {
    pub enabled: bool,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub friction: f32,
    pub velocity: Vec3,
}

impl Default for FreeFlightController {
    fn default() -> Self {
        Self {
            enabled: false,
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            speed: 12.0,
            sensitivity: FPS_MOUSE_SENSITIVITY,
            acceleration: 48.0,
            deceleration: 28.0,
            friction: 0.12,
            velocity: Vec3::ZERO,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LocalPlayer;

#[derive(Clone, Copy, Debug, Default)]
pub struct RemotePlayer;

#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerNameLabel;

pub fn forward_from_angles(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(sy * cp, sp, -cy * cp).normalize_or_zero()
}

pub fn right_from_yaw(yaw: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    Vec3::new(cy, 0.0, sy).normalize_or_zero()
}
