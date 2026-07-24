use super::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum RendererProfile {
    Low,
    Medium,
    High,
    Ultra,
}

impl Default for RendererProfile {
    fn default() -> Self { Self::Medium }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum GlobalIlluminationMode {
    Off,
    Ambient,
    Ddgi,
}

impl Default for GlobalIlluminationMode {
    fn default() -> Self { Self::Ambient }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ToneMapper {
    Off,
    Aces,
    Neutral,
    Reinhard,
}

impl ToneMapper {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Aces,
            Self::Aces => Self::Neutral,
            Self::Neutral => Self::Reinhard,
            Self::Reinhard => Self::Off,
        }
    }

    pub(crate) fn shader_value(self) -> f32 {
        match self {
            Self::Off => 0.0,
            Self::Aces => 1.0,
            Self::Neutral => 2.0,
            Self::Reinhard => 3.0,
        }
    }
}

impl Default for ToneMapper {
    fn default() -> Self { Self::Aces }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectFallbackPolicy {
    pub allow_fallbacks: bool,
    pub profile: RendererProfile,
}

impl Default for EffectFallbackPolicy {
    fn default() -> Self {
        Self { allow_fallbacks: true, profile: RendererProfile::Medium }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostProcessing {
    pub bloom: Bloom,
    pub depth_of_field: DepthOfField,
    pub exposure: f32,
    pub gamma: f32,
    #[serde(default)]
    pub tone_mapper: ToneMapper,
    pub gi_mode: GlobalIlluminationMode,
    pub fallback_policy: EffectFallbackPolicy,
}

impl Default for PostProcessing {
    fn default() -> Self {
        Self {
            bloom: Bloom::default(),
            depth_of_field: DepthOfField::default(),
            exposure: 1.0,
            gamma: 2.2,
            tone_mapper: ToneMapper::default(),
            gi_mode: GlobalIlluminationMode::default(),
            fallback_policy: EffectFallbackPolicy::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CloudProfile {
    pub coverage: f32,
    pub density: f32,
    pub speed: f32,
}

impl Default for CloudProfile {
    fn default() -> Self {
        Self { coverage: 0.4, density: 0.5, speed: 1.0 }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VolumetricCloud {
    pub enabled: bool,
    pub profile: CloudProfile,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Atmosphere {
    pub sun_direction: Vec3,
    pub sky_tint: Vec3,
    pub ground_tint: Vec3,
    pub intensity: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(0.0, 1.0, 0.0),
            sky_tint: Vec3::new(0.45, 0.65, 1.0),
            ground_tint: Vec3::new(0.35, 0.30, 0.25),
            intensity: 1.0,
        }
    }
}
