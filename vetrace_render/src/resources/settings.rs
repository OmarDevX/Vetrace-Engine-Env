use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowFilterMode {
    /// One comparison tap. Fastest, but visibly aliased/pixelated.
    Hard,
    /// Percentage-closer filtering with a fixed radius. Stable general-purpose softening.
    Pcf,
    /// Percentage-closer soft shadows. Keeps contact edges tighter and grows the penumbra with distance.
    Pcss,
    /// Hybrid EVSM: keep the nearest cascade on depth PCF/PCSS for crisp contacts,
    /// render far cascades into exponential moment textures, blur them with separable
    /// horizontal/vertical passes, then sample the blurred moments in the scene pass.
    EvsmBlurred,
}

impl ShadowFilterMode {
    pub fn shader_value(self) -> f32 {
        match self {
            Self::Hard => 0.0,
            Self::Pcf => 1.0,
            Self::Pcss => 2.0,
            Self::EvsmBlurred => 3.0,
        }
    }

    pub fn uses_soft_radius(self) -> bool {
        !matches!(self, Self::Hard)
    }
}

impl Default for ShadowFilterMode {
    fn default() -> Self { Self::Pcss }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientOcclusionMode {
    /// Disable screen-space ambient occlusion completely. No AO targets or passes are allocated.
    Off,
    /// Lightweight depth-based SSAO pass followed by a small depth-aware blur and composite.
    Ssao,
}

impl Default for AmbientOcclusionMode {
    fn default() -> Self { Self::Off }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiAliasingMode {
    /// Disable final-image anti-aliasing. This avoids the post-process target and pass.
    Off,
    /// Fast approximate anti-aliasing. One inexpensive full-screen pass with no
    /// multisampled color/depth buffers, making it the default low-cost option.
    Fxaa,
}

impl Default for AntiAliasingMode {
    fn default() -> Self { Self::Fxaa }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentModePreference {
    /// Stable VSync. Prefer this on Linux/Wayland/VRR setups when presentation
    /// tearing or flicker is more important than lowest possible latency.
    Vsync,
    /// Prefer low latency when available, but fall back to VSync if the platform
    /// does not expose mailbox/immediate presentation.
    LowLatency,
    /// Request immediate presentation if supported. Can tear/flicker on some
    /// compositors and drivers, so games should expose this as an explicit option.
    Immediate,
    /// Request mailbox presentation if supported. Usually lower-latency than Fifo
    /// without visible tearing, but not available everywhere.
    Mailbox,
    /// Request FIFO presentation directly. This is the mandatory VSync mode.
    Fifo,
}

impl Default for PresentModePreference {
    fn default() -> Self { Self::LowLatency }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterPreference {
    /// Prefer the platform's low-power adapter. On hybrid laptops this normally
    /// means the integrated GPU, which is useful for debugging unstable nouveau/NVK
    /// or proprietary-driver setups without changing renderer code.
    LowPower,
    /// Prefer the platform's high-performance adapter. On hybrid laptops this
    /// normally means the discrete GPU.
    HighPerformance,
}

impl AdapterPreference {
    #[cfg(feature = "wgpu_render")]
    pub fn wgpu_power_preference(self) -> wgpu::PowerPreference {
        match self {
            Self::LowPower => wgpu::PowerPreference::LowPower,
            Self::HighPerformance => wgpu::PowerPreference::HighPerformance,
        }
    }
}

impl Default for AdapterPreference {
    fn default() -> Self { Self::HighPerformance }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderSettings {
    pub clear_color: [f32; 4],
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub draw_bounds: bool,
    pub draw_names: bool,
    /// Whether window render targets should lock/grab the cursor for FPS/gameplay input.
    /// Editors normally set this to false so the OS cursor can pick/manipulate objects.
    pub cursor_grab: bool,
    /// Whether window render targets should show the OS cursor.
    pub cursor_visible: bool,
    /// Optional frame time supplied by a game/runtime. Fallback renderers use
    /// this for shader fallback animation; GPU renderers can bind it as a
    /// standard uniform.
    pub time_seconds: f32,
    /// Window presentation policy for GPU window targets. This is a specific
    /// renderer knob, not a quality preset. Games can expose --vsync/--no-vsync
    /// or choose their own latency/tearing policy.
    pub present_mode: PresentModePreference,
    /// GPU adapter selection hint for WGPU window targets. This is a specific
    /// renderer knob, not a quality preset. Games can expose it as debug flags
    /// like --integrated-gpu / --discrete-gpu for hybrid laptop diagnostics.
    pub adapter_preference: AdapterPreference,
    /// Final-image anti-aliasing mode. FXAA is intentionally the default because
    /// it costs one small full-screen pass and does not multiply scene/depth memory.
    /// Screen-space UI and egui are rendered afterward so they stay crisp.
    pub anti_aliasing_mode: AntiAliasingMode,
    /// Directional shadow-map resolution used by the WGPU backend when a
    /// directional light explicitly opts into shadows. Lower this first if
    /// soft shadows feel expensive. Rounded/clamped by the backend.
    pub shadow_map_size: u32,
    /// Maximum number of already-expanded vertices submitted to the directional
    /// shadow pass per frame. This is a safety cap for heavy GLB scenes; zero
    /// disables shadow rendering without disabling lighting.
    pub shadow_max_vertices: u32,
    /// Maximum camera-relative distance for objects to cast directional shadows.
    /// Values <= 0 disable this cull. This keeps free-flight inspections from
    /// re-shadowing an entire huge imported scene every frame.
    pub shadow_max_distance: f32,
    /// Soft-shadow filter radius in shadow-map texels. Zero behaves like hard
    /// shadows even when the light uses ShadowMode::Soft. PCSS can expand this
    /// radius per pixel for contact-hardening soft shadows.
    pub shadow_soft_radius: f32,
    /// Constant receiver depth bias for directional shadows. This is combined
    /// with slope-scale and normal-offset bias in the WGPU backend.
    pub shadow_bias: f32,
    /// Slope-scale receiver bias multiplier. Higher values reduce acne on
    /// grazing surfaces; too high causes peter-panning/detached shadows.
    pub shadow_slope_bias: f32,
    /// World-space normal offset used before projecting receivers into shadow
    /// space. This fights self-shadow acne without needing a huge depth bias.
    pub shadow_normal_bias: f32,
    /// Number of cascades for the primary directional shadow map. Values are
    /// clamped to 1..=4 by the backend. Use 3 or 4 for large outdoor scenes.
    pub shadow_cascade_count: u32,
    /// Selects the directional shadow filtering algorithm used when a light opts into
    /// `ShadowMode::Soft`. Use `EvsmBlurred` when cascaded depth shadows still look
    /// visibly pixelated and you want a stronger blur/smoothing pass.
    pub shadow_filter_mode: ShadowFilterMode,
    /// PCF/PCSS quality level. 1 = 4 Poisson taps, 2 = 8 taps, 3+ = 12 taps.
    /// Higher levels are smoother but cost more shadow texture reads.
    pub shadow_pcf_quality: u32,
    /// Enables approximate PCSS/contact-hardening soft shadows for old configs.
    /// New code should prefer `shadow_filter_mode = ShadowFilterMode::Pcss`.
    pub shadow_pcss: bool,
    /// Approximate directional light source radius used by PCSS, in shadow texels.
    pub shadow_pcss_light_radius: f32,
    /// Separable blur radius in shadow-map texels for `ShadowFilterMode::EvsmBlurred`.
    /// Higher values smooth pixelated edges more but cost more in the EVSM blur passes.
    pub shadow_evsm_blur_radius: f32,
    /// EVSM warp exponent. The WGPU backend clamps this to a half-float-safe range
    /// when using its Rgba16Float moment texture. Values around 4..5 are practical.
    pub shadow_evsm_exponent: f32,
    /// Optional screen-space ambient occlusion mode. `Ssao` is a clean post-process
    /// path: scene color is rendered offscreen, depth is sampled to estimate occlusion,
    /// the AO is blurred, then composited before UI overlays.
    pub ambient_occlusion_mode: AmbientOcclusionMode,
    /// SSAO sample radius in screen pixels. Higher values reach farther into corners
    /// but can halo on thin geometry.
    pub ssao_radius_pixels: f32,
    /// SSAO darkening strength. 0 disables visible AO even when the pass is active.
    pub ssao_intensity: f32,
    /// Depth bias used by the SSAO compare, in post-projection depth units.
    pub ssao_bias: f32,
    /// SSAO sample count. Clamped by the backend to 4..=12.
    pub ssao_sample_count: u32,
    /// Small depth-aware blur radius in pixels. Values around 1..2 are cheap and stable.
    pub ssao_blur_radius: f32,
    /// Maximum runtime reflection-probe capture resolution. Individual probes
    /// may request less; the backend rounds to a supported power of two.
    pub reflection_max_capture_resolution: u32,
    /// Number of cubemap faces a probe may capture in one frame. Values above
    /// one reduce initial bake latency but increase that frame's GPU cost. Zero pauses face capture.
    pub reflection_capture_faces_per_frame: u32,
    /// GGX importance-sampling count used for each nonzero reflection mip.
    /// Higher values reduce noise at the cost of probe-filtering time.
    pub reflection_prefilter_sample_count: u32,
    /// Maximum number of different probes allowed to perform capture/filter work in one frame. Zero pauses all runtime capture work.
    pub reflection_capture_probe_budget_per_frame: u32,
    /// Number of prefilter mip levels a selected probe may process in one frame. Zero pauses filtering.
    pub reflection_prefilter_mips_per_frame: u32,
    /// Maximum number of runtime capture probes kept resident in the fixed GPU pool. Zero streams all runtime captures out.
    pub reflection_max_resident_runtime_probes: u32,
    /// Runtime capture probes farther than this from the camera are streamed out.
    /// Zero or negative values disable the distance limit.
    pub reflection_capture_distance_limit: f32,
    /// Cell size used by the CPU reflection-probe spatial index.
    pub reflection_probe_grid_cell_size: f32,
    /// Cache per-object shadow vertex buffers when an object's mesh/shape and
    /// transform signature did not change. This avoids rebuilding/uploading
    /// large static GLB shadow casters every frame while still updating moved
    /// entities automatically. Disable this for debugging if you mutate mesh
    /// asset data in-place without changing handles.
    pub shadow_cache_geometry: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            clear_color: [0.02, 0.02, 0.03, 1.0],
            width: 1280,
            height: 720,
            title: "Vetrace".to_string(),
            draw_bounds: true,
            draw_names: false,
            cursor_grab: true,
            cursor_visible: false,
            time_seconds: 0.0,
            present_mode: PresentModePreference::default(),
            adapter_preference: AdapterPreference::default(),
            anti_aliasing_mode: AntiAliasingMode::default(),
            shadow_map_size: 1024,
            shadow_max_vertices: 120_000,
            shadow_max_distance: 100.0,
            shadow_soft_radius: 2.0,
            shadow_bias: 0.0015,
            shadow_slope_bias: 1.35,
            shadow_normal_bias: 0.025,
            shadow_cascade_count: 3,
            shadow_filter_mode: ShadowFilterMode::Pcf,
            shadow_pcf_quality: 2,
            shadow_pcss: true,
            shadow_pcss_light_radius: 3.0,
            shadow_evsm_blur_radius: 3.0,
            shadow_evsm_exponent: 5.0,
            ambient_occlusion_mode: AmbientOcclusionMode::Off,
            ssao_radius_pixels: 6.0,
            ssao_intensity: 1.25,
            ssao_bias: 0.0025,
            ssao_sample_count: 8,
            ssao_blur_radius: 1.5,
            reflection_max_capture_resolution: 512,
            reflection_capture_faces_per_frame: 1,
            reflection_prefilter_sample_count: 64,
            reflection_capture_probe_budget_per_frame: 1,
            reflection_prefilter_mips_per_frame: 1,
            reflection_max_resident_runtime_probes: 8,
            reflection_capture_distance_limit: 0.0,
            reflection_probe_grid_cell_size: 12.0,
            shadow_cache_geometry: true,
        }
    }
}

#[path = "settings/sections.rs"]
mod sections;

pub use sections::{
    RenderAmbientOcclusionSettings, RenderPresentationSettings, RenderReflectionSettings,
    RenderShadowSettings,
};
