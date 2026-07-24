use serde::{Deserialize, Serialize};

use super::ShooterGraphicsProfile;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShooterJoinMenuRole {
    Panel,
    Title,
    NameField,
    JoinButton,
    Hint,
}

#[derive(Clone, Copy, Debug)]
pub struct ShooterJoinMenuWidget {
    pub role: ShooterJoinMenuRole,
}

#[derive(Clone, Debug)]
pub struct ShooterJoinMenu {
    pub active: bool,
    pub name: String,
    pub focused: bool,
    pub status: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuPage { Home, Servers, Maps, Shop, Mods, Settings }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainMenuAction {
    Play,
    Servers,
    RefreshServers,
    HostServer,
    PreviousServer,
    NextServer,
    JoinServer,
    Maps,
    Shop,
    Mods,
    Settings,
    ClosePage,
    PreviousMap,
    NextMap,
    PreviousBotDifficulty,
    NextBotDifficulty,
    BotCountDown,
    BotCountUp,
    MaxPlayersDown,
    MaxPlayersUp,
    ToggleMod,
    PreviousMod,
    NextMod,
    ReloadMod,
    ToggleVignette,
    ToggleVolumetricFog,
    CycleGraphics,
    ToggleVsync,
    SensitivityDown,
    SensitivityUp,
    VolumeDown,
    VolumeUp,
    ResetSettings,
    RandomizePlayer,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ShooterGameSettings {
    pub graphics_profile: ShooterGraphicsProfile,
    pub vsync: bool,
    pub vignette: bool,
    pub volumetric_fog: bool,
    pub mouse_sensitivity: f32,
    pub master_volume: f32,
}

impl Default for ShooterGameSettings {
    fn default() -> Self {
        Self {
            graphics_profile: ShooterGraphicsProfile::Balanced,
            vsync: true,
            vignette: true,
            volumetric_fog: false,
            mouse_sensitivity: super::FPS_MOUSE_SENSITIVITY,
            master_volume: 0.8,
        }
    }
}

impl ShooterGameSettings {
    pub fn normalized(mut self) -> Self {
        self.mouse_sensitivity = self.mouse_sensitivity.clamp(0.0004, 0.012);
        self.master_volume = self.master_volume.clamp(0.0, 1.0);
        self
    }

    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str::<Self>(&json).ok())
            .unwrap_or_default()
            .normalized()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.clone().normalized()).map_err(|error| error.to_string())?;
        std::fs::write(path, json).map_err(|error| error.to_string())
    }

    fn path() -> std::path::PathBuf {
        std::env::var_os("VETRACE_SETTINGS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(".vetrace"))
            .join("simple_shooter_settings.json")
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MainMenuWidget {
    pub action: Option<MainMenuAction>,
    pub page_only: bool,
}

impl MainMenuWidget {
    pub fn decoration() -> Self { Self { action: None, page_only: false } }
    pub fn button(action: MainMenuAction) -> Self { Self { action: Some(action), page_only: false } }
    pub fn page_button(action: MainMenuAction) -> Self { Self { action: Some(action), page_only: true } }
    pub fn page_decoration() -> Self { Self { action: None, page_only: true } }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MainMenuPreviewPlayer;

#[derive(Clone, Copy, Debug, Default)]
pub struct MainMenuPreviewStage;

#[derive(Clone, Copy, Debug, Default)]
pub struct MainMenuPreviewOutline;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseMenuAction {
    Resume, Settings, Back, LeaveToMainMenu, Quit,
    CycleGraphics, ToggleVsync, ToggleVignette, ToggleVolumetricFog,
    SensitivityDown, SensitivityUp, VolumeDown, VolumeUp,
}

#[derive(Clone, Copy, Debug)]
pub struct PauseMenuWidget {
    pub action: Option<PauseMenuAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PauseMenuPage { #[default] Home, Settings }

#[derive(Clone, Copy, Debug, Default)]
pub struct PauseMenuState { pub active: bool, pub page: PauseMenuPage }

#[derive(Clone, Debug)]
pub struct MainMenuState {
    pub active: bool,
    pub page: MainMenuPage,
    pub color_roll: u64,
    pub selected_map: usize,
    pub selected_bot_difficulty: super::gameplay::BotDifficulty,
    pub selected_bot_count: u16,
    pub selected_max_players: u16,
    pub mod_enabled: bool,
    pub selected_mod: usize,
    pub vignette_enabled: bool,
    pub status: String,
    pub selected_server: usize,
    pub server_summary: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyAction {
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

#[derive(Clone, Copy, Debug)]
pub struct LobbyWidget {
    pub action: Option<LobbyAction>,
    pub host_control: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LeaderboardWidget;

#[derive(Clone, Copy, Debug, Default)]
pub struct RoundResultsWidget;

#[derive(Clone, Copy, Debug, Default)]
pub struct KillcamWidget;

#[derive(Clone, Copy, Debug, Default)]
pub struct HealthHudWidget;

#[derive(Clone, Copy, Debug)]
pub struct CrosshairPart {
    /// Horizontal part if true, vertical part if false. Kept game-side so the
    /// renderer does not know about FPS HUD policy.
    pub horizontal: bool,
}
