// Public bake configuration and reporting types.

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BakedLightingBakeConfig {
    pub source_name: String,
    /// Square texture resolution allocated to each receiver before its
    /// `resolution_scale` is applied.
    pub lightmap_resolution: u32,
    /// Minimum world-space texel density. Large receivers automatically receive
    /// larger tiles so shadows do not disappear on floors and long walls.
    /// Set to 0 to keep strictly fixed per-object resolutions.
    #[serde(default = "default_lightmap_texels_per_unit")]
    pub lightmap_texels_per_unit: f32,
    /// Small coverage-aware bake filter. It smooths aliased shadow edges without
    /// allowing one UV chart to bleed into another. Zero disables filtering.
    #[serde(default = "default_lightmap_filter_radius")]
    pub lightmap_filter_radius: u32,
    pub atlas_padding: u32,
    pub probe_counts: [u32; 3],
    pub probe_rays: u32,
    pub probe_bounds_padding: f32,
    pub environment_radiance: Vec3,
    /// Number of diffuse probe-lighting iterations. One preserves the original
    /// single-bounce bake; higher values approximate additional diffuse bounces.
    #[serde(default = "default_indirect_bounces")]
    pub indirect_bounces: u32,
    /// Energy retained by each bounce after the first. Values below one keep the
    /// iterative probe solve stable while allowing brighter enclosed interiors.
    #[serde(default = "default_indirect_bounce_decay")]
    pub indirect_bounce_decay: f32,
    /// Final multiplier applied to baked indirect irradiance in lightmaps.
    pub indirect_intensity: f32,
    pub lightmap_intensity: f32,
    /// Upper clamp applied before conversion to RGBA16F. This protects against
    /// invalid runaway energy; it no longer controls an RGBM decode range.
    pub max_baked_radiance: f32,
    pub surface_bias: f32,
}

fn default_lightmap_texels_per_unit() -> f32 { 8.0 }
fn default_lightmap_filter_radius() -> u32 { 1 }
fn default_indirect_bounces() -> u32 { 1 }
fn default_indirect_bounce_decay() -> f32 { 0.65 }

impl Default for BakedLightingBakeConfig {
    fn default() -> Self {
        Self {
            source_name: "scene".to_string(),
            lightmap_resolution: 64,
            lightmap_texels_per_unit: default_lightmap_texels_per_unit(),
            lightmap_filter_radius: default_lightmap_filter_radius(),
            atlas_padding: 4,
            probe_counts: [8, 4, 8],
            probe_rays: 48,
            probe_bounds_padding: 1.0,
            environment_radiance: Vec3::new(0.035, 0.045, 0.065),
            indirect_bounces: default_indirect_bounces(),
            indirect_bounce_decay: default_indirect_bounce_decay(),
            indirect_intensity: 1.0,
            lightmap_intensity: 1.0,
            max_baked_radiance: 8.0,
            surface_bias: 0.003,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BakedLightingBakeReport {
    pub receiver_count: usize,
    pub baked_receiver_count: usize,
    pub skipped_receiver_count: usize,
    pub triangle_count: usize,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub min_lightmap_resolution: u32,
    pub max_lightmap_resolution: u32,
    pub probe_count: usize,
    pub output_bytes: u64,
}
