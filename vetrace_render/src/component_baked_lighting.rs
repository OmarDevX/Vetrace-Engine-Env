use super::*;

/// Marks static renderable geometry as a baked-lightmap receiver and occluder.
///
/// The bake is explicit. Merely adding this component never starts a bake.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BakedLightmapReceiver {
    pub enabled: bool,
    /// Per-object multiplier applied to the configured lightmap resolution.
    pub resolution_scale: f32,
    /// Disable the runtime directional/ambient path for this object once a valid
    /// lightmap is loaded, avoiding duplicate lighting and unnecessary loops.
    pub static_lighting_only: bool,
    /// Keep runtime point/spot lights for transient effects such as muzzle flashes
    /// and emissive bullet trails. Disable for a strict baked-only receiver.
    pub preserve_local_lights: bool,
}

impl Default for BakedLightmapReceiver {
    fn default() -> Self {
        Self {
            enabled: true,
            resolution_scale: 1.0,
            static_lighting_only: true,
            preserve_local_lights: false,
        }
    }
}

/// Opts a renderable into the baked directional probe volume.
/// Intended for moving objects such as players, bots, doors, and pickups.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BakedLightProbeReceiver {
    pub enabled: bool,
    pub intensity: f32,
}

impl Default for BakedLightProbeReceiver {
    fn default() -> Self { Self { enabled: true, intensity: 1.0 } }
}

/// Transient renderer-owned marker used only by the probe-grid debug view.
/// It is never included in a bake and carries no gameplay or physics behavior.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct BakedLightProbeDebugMarker;
