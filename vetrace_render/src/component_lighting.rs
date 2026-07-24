use super::*;

/// Generic renderer-side outline request.
///
/// This is not an editor or shooter component. Games/editors attach it to any
/// renderable entity, and the active backend decides whether to draw a real GPU
/// outline pass or a fallback expanded wire silhouette.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outline {
    pub enabled: bool,
    pub color: Vec3,
    pub thickness: f32,
}

impl Default for Outline {
    fn default() -> Self {
        Self { enabled: true, color: Vec3::splat(0.02), thickness: 0.08 }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct CameraAttachment {
    pub camera: Option<vetrace_core::Entity>,
    pub offset: Vec3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bloom {
    pub enabled: bool,
    pub threshold: f32,
    pub intensity: f32,
    pub radius: f32,
}

impl Default for Bloom {
    fn default() -> Self {
        Self { enabled: false, threshold: 1.0, intensity: 0.5, radius: 1.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepthOfField {
    pub enabled: bool,
    pub focus_distance: f32,
    pub aperture: f32,
    pub focal_length: f32,
}

impl Default for DepthOfField {
    fn default() -> Self {
        Self { enabled: false, focus_distance: 10.0, aperture: 2.8, focal_length: 50.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolumetricFog {
    pub enabled: bool,
    pub color: Vec3,
    pub density: f32,
    pub anisotropy: f32,
}

impl Default for VolumetricFog {
    fn default() -> Self {
        Self { enabled: false, color: Vec3::splat(0.6), density: 0.01, anisotropy: 0.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum ShadowMode {
    None,
    Hard,
    Soft,
}

impl Default for ShadowMode {
    fn default() -> Self { Self::None }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub shadow_mode: ShadowMode,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(-0.3, -1.0, -0.2),
            color: Vec3::ONE,
            intensity: 1.0,
            shadow_mode: ShadowMode::default(),
        }
    }
}

/// Punctual point light component.
///
/// This is renderer-facing but backend-agnostic. glTF `KHR_lights_punctual`
/// point lights map directly to this component. `range = None` means the light
/// has no explicit cutoff distance and the backend may use physically-inspired
/// inverse-square attenuation with a practical clamp.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PointLight {
    pub color: Vec3,
    pub intensity: f32,
    pub range: Option<f32>,
    pub shadow_mode: ShadowMode,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: Vec3::ONE,
            intensity: 1.0,
            range: None,
            shadow_mode: ShadowMode::default(),
        }
    }
}

/// Opt-in approximation that lets an emissive material illuminate nearby geometry.
///
/// The renderer samples one or more shadowless point lights along `local_axis`.
/// Light color and brightness come from the entity's `Material::emissive`
/// value, while this component controls the multiplier, range, and sampling.
/// Keeping this explicit avoids turning every decorative emissive material into
/// a runtime light and makes short-lived effects such as tracers inexpensive.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct EmissiveLightEmitter {
    pub enabled: bool,
    /// Multiplied by the strongest emissive color channel.
    pub intensity: f32,
    /// Point-light cutoff distance in world units.
    pub range: f32,
    /// Local-space line along which point-light samples are distributed.
    pub local_axis: Vec3,
    /// Total local-space sample span. Zero creates a single-position emitter.
    pub length: f32,
    /// Number of point-light samples. Extraction clamps this to 1..=4.
    pub samples: u8,
}

impl Default for EmissiveLightEmitter {
    fn default() -> Self {
        Self {
            enabled: true,
            intensity: 1.0,
            range: 4.0,
            local_axis: Vec3::Z,
            length: 0.0,
            samples: 1,
        }
    }
}


/// Rectangular diffuse emitter used only by the offline/runtime-explicit baked
/// lighting pass.
///
/// The rectangle lies in the entity's local XZ plane and emits along local +Y.
/// Its transform rotates and scales the rectangle into world space. Normal game
/// rendering does not evaluate this light, so loading a bake has no per-frame
/// light-loop or shadow-map cost.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BakedRectAreaLight {
    pub enabled: bool,
    /// Linear-light emitter color.
    pub color: Vec3,
    /// Emitted radiance multiplier used by the CPU baker.
    pub intensity: f32,
    /// Unscaled rectangle width along local X.
    pub width: f32,
    /// Unscaled rectangle height along local Z.
    pub height: f32,
    /// Stratified shadow samples per shaded point. The baker clamps this to
    /// 1..=64. Higher values produce smoother penumbrae at a longer bake time.
    pub samples: u32,
    /// When false, the rectangle emits only along local +Y.
    pub two_sided: bool,
}

impl Default for BakedRectAreaLight {
    fn default() -> Self {
        Self {
            enabled: true,
            color: Vec3::ONE,
            intensity: 12.0,
            width: 1.0,
            height: 1.0,
            samples: 16,
            two_sided: false,
        }
    }
}

/// Punctual spot light component.
///
/// Direction is local-space by design. For glTF imports, spot lights emit along
/// local -Z, then `build_render_frame` rotates it by the entity/global transform.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
    pub range: Option<f32>,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
    pub shadow_mode: ShadowMode,
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            direction: Vec3::new(0.0, 0.0, -1.0),
            color: Vec3::ONE,
            intensity: 1.0,
            range: None,
            inner_cone_angle: 0.0,
            outer_cone_angle: std::f32::consts::FRAC_PI_4,
            shadow_mode: ShadowMode::default(),
        }
    }
}
