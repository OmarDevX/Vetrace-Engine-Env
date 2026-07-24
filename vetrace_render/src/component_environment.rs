use super::*;

/// Box-projection policy for local reflection probes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ReflectionProbeParallaxMode {
    Disabled,
    BoxProjection,
}

impl Default for ReflectionProbeParallaxMode {
    fn default() -> Self { Self::BoxProjection }
}

/// Determines how a reflection probe obtains its cubemap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ReflectionProbeCaptureMode {
    /// Use only `primary` / `secondary` cubemap assets supplied by the game.
    Imported,
    /// Capture the scene once after the probe becomes active or its revision changes.
    Baked,
    /// Capture when `capture_revision` is incremented through `request_capture()`.
    OnDemand,
    /// Recapture automatically according to `update_interval_seconds`.
    Realtime,
}

impl Default for ReflectionProbeCaptureMode {
    fn default() -> Self { Self::Imported }
}


/// Controls when scene changes invalidate an existing runtime/baked capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ReflectionProbeInvalidationMode {
    /// Only `request_capture()` and the normal realtime interval can recapture.
    Manual,
    /// Recapture when relevant render objects, lights, or the global environment change.
    SceneChanges,
}

impl Default for ReflectionProbeInvalidationMode {
    fn default() -> Self { Self::Manual }
}

/// Policy for custom-shader objects during cubemap capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ReflectionProbeCustomMaterialCaptureMode {
    /// Omit custom-shader objects from the cubemap.
    Exclude,
    /// Capture geometry through its standard `Material` fallback.
    MaterialFallback,
    /// Compile the custom fragment shader for the linear HDR capture target.
    Shader,
}

impl Default for ReflectionProbeCustomMaterialCaptureMode {
    fn default() -> Self { Self::MaterialFallback }
}

/// Local reflection volume backed by one or two cubemap assets.
///
/// The owning entity's world transform positions and rotates the influence box.
/// `half_extents` are expressed in the probe's local space.  `secondary` and
/// `transition` provide an explicit, artifact-free crossfade when a probe is
/// replaced or when a room changes lighting state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ReflectionProbe {
    pub enabled: bool,
    pub primary: Option<CubemapHandle>,
    pub secondary: Option<CubemapHandle>,
    pub transition: f32,
    pub half_extents: Vec3,
    pub capture_offset: Vec3,
    pub blend_distance: f32,
    pub intensity: f32,
    pub priority: i32,
    pub parallax_mode: ReflectionProbeParallaxMode,
    pub capture_mode: ReflectionProbeCaptureMode,
    /// Power-of-two capture face resolution. The backend clamps this to the
    /// renderer-wide `reflection_max_capture_resolution` quality limit and
    /// filters the result into its shared reflection pool.
    pub capture_resolution: u32,
    /// Near and far planes used by the six internal capture cameras.
    pub capture_near: f32,
    pub capture_far: f32,
    /// Crossfade duration from the previous completed capture to the new one.
    pub transition_seconds: f32,
    /// Realtime recapture interval. Values at or below zero request the fastest
    /// supported update cadence, still amortized by the renderer's work budget.
    pub update_interval_seconds: f32,
    /// Incremented by `request_capture`; extracted as a pure renderer request.
    pub capture_revision: u32,
    /// Additional scheduler priority used only for runtime capture work.
    pub capture_priority: i32,
    /// Optional automatic invalidation when captured scene content changes.
    pub invalidation_mode: ReflectionProbeInvalidationMode,
    /// Scene changes must remain stable for this long before an automatic
    /// recapture begins. This prevents continuously moving objects from
    /// restarting baked/on-demand probes every frame.
    pub invalidation_delay_seconds: f32,
    /// Whether alpha-blended default materials are rendered into the cubemap.
    pub capture_transparent: bool,
    /// Render capture-camera directional shadows before each cubemap face.
    /// This is high quality but intentionally opt-in because it adds shadow
    /// passes to every captured face.
    pub capture_shadows: bool,
    /// How custom-shader materials participate in cubemap capture.
    pub capture_custom_materials: ReflectionProbeCustomMaterialCaptureMode,
    /// Objects on these layers may receive this probe while shading.
    pub include_layers: u32,
    /// Objects on these layers do not receive this probe while shading.
    pub exclude_layers: u32,
    /// Scene layers rendered into runtime/baked cubemap captures. Kept
    /// separate from influence layers so a mirror can sample a probe without
    /// being included in that probe's own capture.
    pub capture_include_layers: u32,
    /// Scene layers omitted only from runtime/baked cubemap captures.
    pub capture_exclude_layers: u32,
}

impl Default for ReflectionProbe {
    fn default() -> Self {
        Self {
            enabled: true,
            primary: None,
            secondary: None,
            transition: 0.0,
            half_extents: Vec3::splat(5.0),
            capture_offset: Vec3::ZERO,
            blend_distance: 1.0,
            intensity: 1.0,
            priority: 0,
            parallax_mode: ReflectionProbeParallaxMode::BoxProjection,
            capture_mode: ReflectionProbeCaptureMode::Imported,
            capture_resolution: 128,
            capture_near: 0.05,
            capture_far: 1_000.0,
            transition_seconds: 0.35,
            update_interval_seconds: 1.0,
            capture_revision: 0,
            capture_priority: 0,
            invalidation_mode: ReflectionProbeInvalidationMode::Manual,
            invalidation_delay_seconds: 0.2,
            capture_transparent: false,
            capture_shadows: false,
            capture_custom_materials: ReflectionProbeCustomMaterialCaptureMode::MaterialFallback,
            include_layers: ALL_RENDER_LAYERS,
            exclude_layers: 0,
            capture_include_layers: ALL_RENDER_LAYERS,
            capture_exclude_layers: 0,
        }
    }
}


impl ReflectionProbe {
    /// Requests a new baked/on-demand capture without requiring renderer access.
    pub fn request_capture(&mut self) {
        self.capture_revision = self.capture_revision.wrapping_add(1);
    }

    /// Starts a local probe cubemap crossfade without dropping the current map.
    pub fn begin_transition(&mut self, next: CubemapHandle) {
        self.enabled = true;
        match self.primary {
            None => {
                self.primary = Some(next);
                self.secondary = None;
                self.transition = 0.0;
            }
            Some(current) if current == next => {
                self.secondary = None;
                self.transition = 0.0;
            }
            Some(_) => {
                self.secondary = Some(next);
                self.transition = 0.0;
            }
        }
    }

    /// Advances the crossfade and promotes the secondary map at completion.
    pub fn advance_transition(&mut self, delta_seconds: f32, duration_seconds: f32) -> bool {
        let Some(next) = self.secondary else {
            self.transition = 0.0;
            return false;
        };
        if duration_seconds <= 0.0 {
            self.primary = Some(next);
            self.secondary = None;
            self.transition = 0.0;
            return true;
        }
        self.transition = (self.transition + delta_seconds.max(0.0) / duration_seconds).clamp(0.0, 1.0);
        if self.transition < 1.0 {
            return false;
        }
        self.primary = Some(next);
        self.secondary = None;
        self.transition = 0.0;
        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_request_advances_revision() {
        let mut probe = ReflectionProbe::default();
        probe.request_capture();
        assert_eq!(probe.capture_revision, 1);
    }

    #[test]
    fn capture_layers_are_independent_from_probe_influence_layers() {
        let probe = ReflectionProbe {
            exclude_layers: 1 << 2,
            capture_exclude_layers: 1 << 5,
            ..ReflectionProbe::default()
        };
        assert_eq!(probe.exclude_layers, 1 << 2);
        assert_eq!(probe.capture_exclude_layers, 1 << 5);
        assert_eq!(probe.include_layers, ALL_RENDER_LAYERS);
        assert_eq!(probe.capture_include_layers, ALL_RENDER_LAYERS);
    }

    #[test]
    fn scene_invalidation_is_opt_in() {
        let probe = ReflectionProbe::default();
        assert_eq!(probe.invalidation_mode, ReflectionProbeInvalidationMode::Manual);
        assert_eq!(probe.invalidation_delay_seconds, 0.2);
        assert_eq!(probe.capture_custom_materials, ReflectionProbeCustomMaterialCaptureMode::MaterialFallback);
        assert!(!probe.capture_transparent);
        assert!(!probe.capture_shadows);
    }

    #[test]
    fn probe_crossfade_promotes_secondary() {
        let mut probe = ReflectionProbe {
            primary: Some(CubemapHandle(1)),
            ..ReflectionProbe::default()
        };
        probe.begin_transition(CubemapHandle(2));
        assert_eq!(probe.secondary, Some(CubemapHandle(2)));
        assert!(!probe.advance_transition(0.5, 2.0));
        assert_eq!(probe.transition, 0.25);
        assert!(probe.advance_transition(1.5, 2.0));
        assert_eq!(probe.primary, Some(CubemapHandle(2)));
        assert_eq!(probe.secondary, None);
    }
}
