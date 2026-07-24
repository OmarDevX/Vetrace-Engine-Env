use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

pub const SERVER_AUTHORITY_ID: u64 = 1;
pub const FIRST_REMOTE_PLAYER_ID: u64 = SERVER_AUTHORITY_ID + 1;
pub const DEFAULT_MAX_PLAYERS: u16 = 8;
pub const DEFAULT_BOT_COUNT: u16 = 3;
pub const MAX_BOT_COUNT: u16 = 32;
pub const MAX_PLAYER_LIMIT: u16 = 64;
pub const ROUND_RESULTS_DURATION_SECONDS: f32 = 8.0;
pub const CLIENT_DISCONNECT_TIMEOUT_SECONDS: f32 = 6.0;
pub const DEFAULT_KILL_LIMIT: u32 = 30;
pub const MIN_KILL_LIMIT: u32 = 5;
pub const MAX_KILL_LIMIT: u32 = 500;
pub const KILL_LIMIT_STEP: u32 = 5;
pub const RULE_MULTIPLIER_STEP: f32 = 0.25;
pub const MIN_MOVE_MULTIPLIER: f32 = 0.25;
pub const MAX_MOVE_MULTIPLIER: f32 = 3.0;
pub const MIN_GRAVITY_SCALE: f32 = 0.1;
pub const MAX_GRAVITY_SCALE: f32 = 3.0;
pub const MIN_JUMP_MULTIPLIER: f32 = 0.25;
pub const MAX_JUMP_MULTIPLIER: f32 = 3.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotDifficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

impl BotDifficulty {
    pub fn name(self) -> &'static str {
        match self { Self::Easy => "Easy", Self::Normal => "Normal", Self::Hard => "Hard" }
    }

    pub fn previous(self) -> Self {
        match self { Self::Easy => Self::Hard, Self::Normal => Self::Easy, Self::Hard => Self::Normal }
    }

    pub fn next(self) -> Self {
        match self { Self::Easy => Self::Normal, Self::Normal => Self::Hard, Self::Hard => Self::Easy }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ShooterInput {
    pub movement: Vec2,
    pub yaw: f32,
    pub pitch: f32,
    pub fire: bool,
    pub jump: bool,
}

impl Default for ShooterInput {
    fn default() -> Self {
        Self { movement: Vec2::ZERO, yaw: 0.0, pitch: 0.0, fire: false, jump: false }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BulletTrail {
    pub ttl: f32,
    pub from: Vec3,
    pub to: Vec3,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShooterAudioListener;

#[derive(Clone, Debug, Default)]
pub struct ShooterStats {
    pub shots_fired: u64,
    pub hits: u64,
    pub deaths: u64,
}

#[derive(Clone, Debug)]
pub struct ShooterKillcamState {
    pub death_number: Option<u32>,
    pub camera_position: Vec3,
    pub camera_target: Vec3,
}

impl Default for ShooterKillcamState {
    fn default() -> Self {
        Self { death_number: None, camera_position: Vec3::ZERO, camera_target: Vec3::ZERO }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRules {
    pub map_index: u8,
    pub mod_index: u8,
    pub mod_enabled: bool,
    pub bots_enabled: bool,
    pub bot_count: u16,
    pub bot_difficulty: BotDifficulty,
    pub max_players: u16,
    pub move_speed_multiplier: f32,
    pub gravity_scale: f32,
    pub jump_multiplier: f32,
    pub kill_limit: u32,
}

impl Default for SessionRules {
    fn default() -> Self {
        Self {
            map_index: 0,
            mod_index: 0,
            mod_enabled: false,
            bots_enabled: false,
            bot_count: DEFAULT_BOT_COUNT,
            bot_difficulty: BotDifficulty::Normal,
            max_players: DEFAULT_MAX_PLAYERS,
            move_speed_multiplier: 1.0,
            gravity_scale: 1.0,
            jump_multiplier: 1.0,
            kill_limit: DEFAULT_KILL_LIMIT,
        }
    }
}

impl SessionRules {
    pub fn normalized(mut self) -> Self {
        self.move_speed_multiplier = self.move_speed_multiplier.clamp(MIN_MOVE_MULTIPLIER, MAX_MOVE_MULTIPLIER);
        self.gravity_scale = self.gravity_scale.clamp(MIN_GRAVITY_SCALE, MAX_GRAVITY_SCALE);
        self.jump_multiplier = self.jump_multiplier.clamp(MIN_JUMP_MULTIPLIER, MAX_JUMP_MULTIPLIER);
        self.kill_limit = self.kill_limit.clamp(MIN_KILL_LIMIT, MAX_KILL_LIMIT);
        self.bot_count = self.bot_count.min(MAX_BOT_COUNT);
        self.max_players = self.max_players.clamp(1, MAX_PLAYER_LIMIT);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreStanding {
    pub name: String,
    pub kills: u32,
    pub deaths: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MatchPhase {
    Lobby,
    Playing,
    Results { remaining_seconds: f32, standings: Vec<ScoreStanding> },
}

impl MatchPhase {
    pub fn is_lobby(&self) -> bool { matches!(self, Self::Lobby) }
    pub fn is_playing(&self) -> bool { matches!(self, Self::Playing) }
    pub fn is_results(&self) -> bool { matches!(self, Self::Results { .. }) }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShooterBot;

#[derive(Clone, Copy, Debug, Default)]
pub struct ShooterMapGeometry;

#[derive(Clone, Copy, Debug, Default)]
pub struct ShooterNavigationObstacle;

#[derive(Clone, Debug)]
pub struct ShooterBotNavigation {
    pub target_id: Option<u64>,
    pub repath_timer: f32,
    pub waypoint_index: usize,
    pub path: Vec<Vec3>,
    pub reaction_timer: f32,
    pub fire_timer: f32,
    pub aim_phase: f32,
}

impl Default for ShooterBotNavigation {
    fn default() -> Self {
        Self {
            target_id: None,
            repath_timer: 0.0,
            waypoint_index: 0,
            path: Vec::new(),
            reaction_timer: 0.0,
            fire_timer: 0.0,
            aim_phase: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShooterMapKind {
    None,
    Lobby,
    Game(u8),
}

#[derive(Clone, Debug)]
pub struct ShooterMapState {
    pub active: ShooterMapKind,
    pub spawn_points: Vec<Vec3>,
    pub validation_error: Option<String>,
}

impl Default for ShooterMapState {
    fn default() -> Self { Self { active: ShooterMapKind::None, spawn_points: Vec::new(), validation_error: None } }
}

#[derive(Clone, Debug)]
pub struct ShooterSession {
    pub phase: MatchPhase,
    pub local_is_admin: bool,
    pub admin_id: u64,
    pub rules: SessionRules,
    pub server_name: String,
    pub controls_open: bool,
}

impl Default for ShooterSession {
    fn default() -> Self {
        Self {
            phase: MatchPhase::Playing,
            local_is_admin: false,
            admin_id: SERVER_AUTHORITY_ID,
            rules: SessionRules::default(),
            server_name: String::new(),
            controls_open: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_rule_bounds_are_centralized() {
        let rules = SessionRules {
            move_speed_multiplier: 99.0,
            gravity_scale: 0.0,
            jump_multiplier: -4.0,
            kill_limit: 1,
            ..SessionRules::default()
        }.normalized();
        assert_eq!(rules.move_speed_multiplier, 3.0);
        assert_eq!(rules.gravity_scale, 0.1);
        assert_eq!(rules.jump_multiplier, 0.25);
        assert_eq!(rules.kill_limit, MIN_KILL_LIMIT);
    }

    #[test]
    fn match_phase_helpers_are_exclusive() {
        assert!(MatchPhase::Lobby.is_lobby());
        assert!(MatchPhase::Playing.is_playing());
        assert!(MatchPhase::Results { remaining_seconds: 1.0, standings: Vec::new() }.is_results());
    }
}
