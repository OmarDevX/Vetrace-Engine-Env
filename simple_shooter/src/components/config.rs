use vetrace_render::{AdapterPreference, PresentModePreference, RenderSettings};

use super::{ShooterGraphicsProfile, DEFAULT_BOT_COUNT, DEFAULT_MAX_PLAYERS};

#[derive(Clone, Debug)]
pub struct ShooterConfig {
    pub mode: ShooterMode,
    pub bind_addr: String,
    pub server_addr: String,
    pub player_name: String,
    pub prompt_player_name_in_ui: bool,
    pub max_frames: Option<usize>,
    pub use_scripted_input: bool,
    /// Start with the optional editor plugin active. Requires the `editor`
    /// cargo feature to actually load the plugin.
    pub editor_enabled: bool,
    /// Enable the optional vetrace_profiler plugin when built with `--features profiler`.
    pub profile_enabled: bool,
    /// Where the profiler UI should be drawn when profiling is enabled.
    pub profile_ui_mode: ShooterProfileUiMode,
    /// Optional scene JSON map exported from vetrace_map_builder.
    pub map_json_path: Option<String>,
    /// Runtime graphics profile selected by the Simple Shooter app/CLI.
    /// This policy intentionally lives game-side; vetrace_render only exposes knobs.
    pub graphics_profile: ShooterGraphicsProfile,
    /// True when a CLI graphics-profile flag was supplied. This lets headless
    /// and bake commands override the saved menu profile intentionally.
    pub graphics_profile_explicit: bool,
    /// When the glTF feature is enabled, load the bundled demo GLB at startup.
    /// Keep this switch game-side so the renderer importer stays reusable.
    pub load_demo_gltf: bool,
    /// Optional fog override for quick renderer testing without changing graphics profile.
    pub force_fog: Option<bool>,
    /// Optional shadow override for renderer debugging.  This stays game-side:
    /// vetrace_render exposes shadow knobs, Simple Shooter owns CLI policy.
    pub force_shadows: Option<bool>,
    /// Optional presentation override. None leaves the renderer default policy;
    /// Some(true) asks for VSync and Some(false) asks for low-latency presentation.
    pub vsync: Option<bool>,
    /// Optional GPU adapter override for hybrid laptop/driver diagnostics.
    /// None leaves the renderer default; Some(LowPower) usually selects the iGPU,
    /// Some(HighPerformance) usually selects the dGPU.
    pub adapter_preference: Option<AdapterPreference>,
    /// Optional Simple Shooter demo effect using vetrace_render's custom post-process API.
    pub post_vignette: bool,
    /// Show the game-owned front end before entering gameplay.
    pub show_main_menu: bool,
    /// Optional manifest-driven Lua mod directory.
    pub mods_dir: Option<String>,
    /// Number of bots requested for offline play and bot-enabled hosted sessions.
    pub bot_count: u16,
    /// Maximum number of human players accepted by a hosted session.
    pub max_players: u16,
    /// UDP port used only for local-network server discovery.
    pub discovery_port: u16,
    /// Explicit tool mode: bake the active map's lightmaps/probes and save them.
    /// Normal gameplay only loads existing `.vlight` files.
    pub bake_lighting: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShooterProfileUiMode {
    Detached,
    Overlay,
    Both,
}

impl Default for ShooterConfig {
    fn default() -> Self {
        Self {
            mode: ShooterMode::Offline,
            bind_addr: "0.0.0.0:3456".to_string(),
            server_addr: "127.0.0.1:3456".to_string(),
            player_name: "Player".to_string(),
            prompt_player_name_in_ui: false,
            max_frames: None,
            use_scripted_input: cfg!(not(feature = "window")),
            editor_enabled: false,
            profile_enabled: false,
            profile_ui_mode: ShooterProfileUiMode::Detached,
            map_json_path: None,
            graphics_profile: ShooterGraphicsProfile::default(),
            graphics_profile_explicit: false,
            load_demo_gltf: true,
            force_fog: None,
            force_shadows: None,
            vsync: None,
            adapter_preference: None,
            post_vignette: false,
            show_main_menu: false,
            mods_dir: None,
            bot_count: DEFAULT_BOT_COUNT,
            max_players: DEFAULT_MAX_PLAYERS,
            discovery_port: 34_557,
            bake_lighting: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShooterMode {
    Offline,
    Host,
    Join,
}

/// Build the render settings Simple Shooter wants before RenderPlugin creates
/// the native window. This must happen before plugin initialization; app setup
/// also writes the same resource so runtime/editor changes still see it.
pub fn shooter_initial_render_settings(config: &ShooterConfig) -> RenderSettings {
    let mut settings = RenderSettings::default();
    settings.title = "Vetrace Simple Shooter".to_string();
    settings.clear_color = [0.01, 0.012, 0.018, 1.0];
    settings.width = 1280;
    settings.height = 720;
    config.graphics_profile.apply_to_render_settings(&mut settings);
    if let Some(vsync) = config.vsync {
        settings.present_mode = if vsync { PresentModePreference::Vsync } else { PresentModePreference::LowLatency };
    }
    if let Some(adapter_preference) = config.adapter_preference {
        settings.adapter_preference = adapter_preference;
    }
    settings
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ShooterBakedLightingSettings {
    pub force_bake: bool,
    pub graphics_profile: ShooterGraphicsProfile,
}
