use serde::{Deserialize, Serialize};

use crate::ProjectPath;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApplicationSettings {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub fullscreen: bool,
    /// Lock/confine the OS cursor while the standalone game window is focused.
    /// Editor preview always overrides this to `false`.
    pub cursor_grab: bool,
    /// Show the OS cursor in standalone game mode. Editor preview always
    /// overrides this to `true`.
    pub cursor_visible: bool,
    pub icon: Option<ProjectPath>,
}

impl Default for ApplicationSettings {
    fn default() -> Self {
        Self {
            title: "Vetrace Game".to_owned(),
            width: 1280,
            height: 720,
            resizable: true,
            fullscreen: false,
            cursor_grab: true,
            cursor_visible: false,
            icon: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeSettings {
    pub main_scene: ProjectPath,
    pub autoload_scripts: Vec<ProjectPath>,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            main_scene: ProjectPath::new("assets/scenes/main.vscene").expect("default project path is valid"),
            autoload_scripts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptingSettings {
    pub language: ScriptLanguage,
    pub hot_reload: bool,
    /// Stop the entire runtime on an uncaught script error. Editor preview
    /// normally leaves this disabled so only the failing script is stopped.
    pub fail_fast: bool,
    pub max_errors_per_frame: u32,
}

impl Default for ScriptingSettings {
    fn default() -> Self {
        Self {
            language: ScriptLanguage::Lua,
            hot_reload: true,
            fail_fast: false,
            max_errors_per_frame: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptLanguage {
    #[default]
    Lua,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderingSettings {
    pub backend: RenderingBackend,
    /// Compatibility switch retained for existing manifests. When
    /// `present_mode` is `Auto`, this selects VSync or low latency.
    pub vsync: bool,
    pub present_mode: PresentMode,
    pub adapter_preference: AdapterPreference,
    pub anti_aliasing: AntiAliasing,
    pub ambient_occlusion: AmbientOcclusion,
    pub ssao_radius_pixels: f32,
    pub ssao_intensity: f32,
    pub ssao_sample_count: u32,
    pub hdr: bool,
    pub msaa_samples: u8,
    pub render_scale: f32,
    pub shadow_quality: ShadowQuality,
    pub shadow_max_distance: f32,
    pub shadow_soft_radius: f32,
    pub shadow_bias: f32,
    pub shadow_slope_bias: f32,
    pub shadow_normal_bias: f32,
    pub shadow_cache_geometry: bool,
    pub gi_mode: GiMode,
}

impl Default for RenderingSettings {
    fn default() -> Self {
        Self {
            backend: RenderingBackend::Auto,
            vsync: true,
            present_mode: PresentMode::Auto,
            adapter_preference: AdapterPreference::HighPerformance,
            anti_aliasing: AntiAliasing::Fxaa,
            ambient_occlusion: AmbientOcclusion::Off,
            ssao_radius_pixels: 6.0,
            ssao_intensity: 1.25,
            ssao_sample_count: 8,
            hdr: true,
            msaa_samples: 1,
            render_scale: 1.0,
            shadow_quality: ShadowQuality::Medium,
            shadow_max_distance: 100.0,
            shadow_soft_radius: 2.0,
            shadow_bias: 0.0015,
            shadow_slope_bias: 1.35,
            shadow_normal_bias: 0.025,
            shadow_cache_geometry: true,
            gi_mode: GiMode::None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderingBackend {
    #[default]
    Auto,
    Wgpu,
    SoftwareSdl,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentMode {
    #[default]
    Auto,
    Vsync,
    LowLatency,
    Immediate,
    Mailbox,
    Fifo,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterPreference {
    LowPower,
    #[default]
    HighPerformance,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasing {
    Off,
    #[default]
    Fxaa,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientOcclusion {
    #[default]
    Off,
    Ssao,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowQuality {
    Off,
    Low,
    #[default]
    Medium,
    High,
    Ultra,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GiMode {
    #[default]
    None,
    Baked,
    Ddgi,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsSettings {
    pub gravity: [f32; 3],
    pub fixed_timestep: f32,
    pub max_substeps: u8,
}

impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            gravity: [0.0, -9.81, 0.0],
            fixed_timestep: 1.0 / 60.0,
            max_substeps: 8,
        }
    }
}
