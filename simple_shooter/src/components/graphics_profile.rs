use serde::{Deserialize, Serialize};
use vetrace_render::{AmbientOcclusionMode, RenderSettings, ShadowFilterMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShooterGraphicsProfile {
    /// Simple Shooter policy for weak GPUs, VMs, old Windows laptops, and battery mode.
    LowSpec,
    /// Simple Shooter default: responsive with cheap shadows and no heavy post effects.
    Balanced,
    /// Simple Shooter visual preset: stronger shadows and SSAO for capable machines.
    HighQuality,
}

impl Default for ShooterGraphicsProfile {
    fn default() -> Self { Self::Balanced }
}

impl ShooterGraphicsProfile {
    pub fn apply_to_render_settings(self, settings: &mut RenderSettings) {
        match self {
            ShooterGraphicsProfile::LowSpec => {
                settings.draw_bounds = false;
                settings.shadow_map_size = 512;
                settings.shadow_max_vertices = 0;
                settings.shadow_max_distance = 0.0;
                settings.shadow_soft_radius = 0.0;
                settings.shadow_bias = 0.0015;
                settings.shadow_slope_bias = 1.0;
                settings.shadow_normal_bias = 0.015;
                settings.shadow_cascade_count = 1;
                settings.shadow_filter_mode = ShadowFilterMode::Hard;
                settings.shadow_pcf_quality = 1;
                settings.shadow_pcss = false;
                settings.shadow_pcss_light_radius = 1.0;
                settings.shadow_evsm_blur_radius = 0.0;
                settings.ambient_occlusion_mode = AmbientOcclusionMode::Off;
                settings.ssao_sample_count = 4;
                settings.ssao_intensity = 0.0;
            }
            ShooterGraphicsProfile::Balanced => {
                settings.draw_bounds = false;
                settings.shadow_map_size = 1024;
                settings.shadow_max_vertices = 45_000;
                settings.shadow_max_distance = 50.0;
                settings.shadow_soft_radius = 0.0;
                settings.shadow_bias = 0.0015;
                settings.shadow_slope_bias = 1.1;
                settings.shadow_normal_bias = 0.02;
                settings.shadow_cascade_count = 1;
                settings.shadow_filter_mode = ShadowFilterMode::Hard;
                settings.shadow_pcf_quality = 1;
                settings.shadow_pcss = false;
                settings.shadow_pcss_light_radius = 1.0;
                settings.shadow_evsm_blur_radius = 0.0;
                settings.ambient_occlusion_mode = AmbientOcclusionMode::Off;
                settings.ssao_sample_count = 4;
                settings.ssao_intensity = 0.0;
            }
            ShooterGraphicsProfile::HighQuality => {
                settings.draw_bounds = true;
                settings.shadow_map_size = 2048;
                settings.shadow_max_vertices = 90_000;
                settings.shadow_max_distance = 75.0;
                settings.shadow_soft_radius = 3.0;
                settings.shadow_bias = 0.0015;
                settings.shadow_slope_bias = 1.35;
                settings.shadow_normal_bias = 0.02;
                settings.shadow_cascade_count = 3;
                settings.shadow_filter_mode = ShadowFilterMode::Pcf;
                settings.shadow_pcf_quality = 2;
                settings.shadow_pcss = true;
                settings.shadow_pcss_light_radius = 3.0;
                settings.shadow_evsm_blur_radius = 3.0;
                settings.shadow_evsm_exponent = 5.0;
                settings.ambient_occlusion_mode = AmbientOcclusionMode::Ssao;
                settings.ssao_radius_pixels = 6.0;
                settings.ssao_intensity = 1.10;
                settings.ssao_bias = 0.0025;
                settings.ssao_sample_count = 8;
                settings.ssao_blur_radius = 1.5;
            }
        }
    }

    pub fn enables_demo_fog_by_default(self) -> bool {
        matches!(self, ShooterGraphicsProfile::HighQuality)
    }
}
