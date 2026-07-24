use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostProcessInput {
    /// Bind the current scene color to the custom full-screen shader.
    SceneColor,
    /// Bind scene color plus sampled scene depth. This uses the same bind layout
    /// as `SceneColor`; shaders can simply ignore depth when they do not need it.
    SceneColorDepth,
}

impl Default for PostProcessInput {
    fn default() -> Self { Self::SceneColor }
}

/// Public, renderer-neutral description of a custom full-screen post-process pass.
///
/// The WGPU backend expects a WGSL module with `vs_main` and `fs_main` entry
/// points. Bind group 0 uses this ABI:
///
/// - binding 0: `texture_2d<f32>` current scene color
/// - binding 1: `sampler` screen sampler
/// - binding 2: `texture_depth_2d` scene depth
/// - binding 3: `CustomPostProcessUniform` uniform buffer
/// - binding 4: optional temporal-history `texture_2d<f32>`; generic passes may ignore it
///
/// This stays engine-side as a specific render feature. Games own the policy:
/// which passes exist, what they are called, how they are exposed in menus/CLI,
/// and which parameter values they use.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomPostProcessPass {
    pub pass_id: String,
    pub asset_path: Option<String>,
    pub wgsl_source: Option<String>,
    pub params: Vec<f32>,
    pub order: i32,
    pub enabled: bool,
    pub input: PostProcessInput,
}

impl Default for CustomPostProcessPass {
    fn default() -> Self {
        Self {
            pass_id: "custom_post_process".to_string(),
            asset_path: None,
            wgsl_source: None,
            params: Vec::new(),
            order: 0,
            enabled: true,
            input: PostProcessInput::SceneColor,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomPostProcessStack {
    pub passes: Vec<CustomPostProcessPass>,
}

/// Stable pass identifier used by the built-in screen-space reflection layer.
pub const SCREEN_SPACE_REFLECTIONS_PASS_ID: &str = "vetrace_screen_space_reflections";

/// Reusable hybrid SSR settings.
///
/// The WGPU backend converts this resource into a normal custom post-process
/// pass internally. Games get a typed, serializable API while the generic
/// custom-pass system remains available for project-specific effects.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenSpaceReflections {
    pub enabled: bool,
    pub intensity: f32,
    pub max_distance: f32,
    pub thickness: f32,
    pub stride: f32,
    pub max_steps: u32,
    pub edge_fade: f32,
    pub start_distance: f32,
    pub origin_bias: f32,
    /// Fraction of `max_distance` where confidence starts fading to the probe.
    pub distance_fade_start: f32,
    /// Minimum hit-surface facing confidence. Higher values reject more grazing hits.
    pub normal_rejection: f32,
    pub max_confidence: f32,
    /// Reuse the previous SSR result to reduce shimmer and short-lived holes.
    pub temporal_enabled: bool,
    /// Maximum history blend for reliable, non-disoccluded pixels.
    pub temporal_weight: f32,
    /// RGB neighborhood expansion used while clamping history.
    pub history_clamp: f32,
    /// Reject history when its color differs strongly from the current result.
    pub disocclusion_threshold: f32,
    pub order: i32,
}

impl Default for ScreenSpaceReflections {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 0.5,
            max_distance: 8.0,
            thickness: 0.22,
            stride: 0.16,
            max_steps: 56,
            edge_fade: 0.10,
            start_distance: 0.24,
            origin_bias: 0.035,
            distance_fade_start: 0.62,
            normal_rejection: 0.08,
            max_confidence: 0.88,
            temporal_enabled: true,
            temporal_weight: 0.18,
            history_clamp: 0.08,
            disocclusion_threshold: 0.22,
            order: 10,
        }
    }
}

impl ScreenSpaceReflections {
    pub fn as_custom_post_process_pass(&self) -> CustomPostProcessPass {
        CustomPostProcessPass {
            pass_id: SCREEN_SPACE_REFLECTIONS_PASS_ID.to_string(),
            wgsl_source: Some(include_str!("../wgpu_window/screen_space_reflections.wgsl").to_string()),
            params: vec![
                self.intensity.max(0.0),
                self.max_distance.max(0.1),
                self.thickness.max(0.001),
                self.stride.max(0.01),
                self.max_steps.clamp(4, 96) as f32,
                self.edge_fade.clamp(0.001, 0.5),
                self.start_distance.max(0.0),
                if self.enabled { 1.0 } else { 0.0 },
                self.origin_bias.max(0.0),
                self.distance_fade_start.clamp(0.0, 0.99),
                self.normal_rejection.clamp(0.0, 0.99),
                self.max_confidence.clamp(0.0, 1.0),
                if self.temporal_enabled { 1.0 } else { 0.0 },
                self.temporal_weight.clamp(0.0, 0.95),
                self.history_clamp.max(0.0),
                self.disocclusion_threshold.max(0.001),
            ],
            order: self.order,
            enabled: self.enabled,
            input: PostProcessInput::SceneColorDepth,
            ..CustomPostProcessPass::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_ssr_builds_stable_custom_pass() {
        let settings = ScreenSpaceReflections {
            enabled: true,
            max_steps: 200,
            ..ScreenSpaceReflections::default()
        };
        let pass = settings.as_custom_post_process_pass();
        assert_eq!(pass.pass_id, SCREEN_SPACE_REFLECTIONS_PASS_ID);
        assert!(pass.enabled);
        assert_eq!(pass.input, PostProcessInput::SceneColorDepth);
        assert_eq!(pass.params.len(), 16);
        assert_eq!(pass.params[4], 96.0);
        assert!(pass.wgsl_source.as_deref().is_some_and(|source| source.contains("depth_continuity")));
    }
}
