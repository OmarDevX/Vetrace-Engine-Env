use super::*;

/// All render layers enabled. Entities without an explicit `RenderLayers`
/// component are treated as belonging to this mask.
pub const ALL_RENDER_LAYERS: u32 = u32::MAX;

/// Optional visibility mask for rasterized objects.
///
/// The main camera currently renders every non-zero layer. A
/// `RenderTextureCamera` applies its own `layer_mask`, which lets games hide the
/// mirror/portal surface, first-person arms, HUD-world meshes, or other objects
/// from a secondary view without teaching the renderer about those concepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderLayers {
    pub mask: u32,
}

impl Default for RenderLayers {
    fn default() -> Self { Self { mask: ALL_RENDER_LAYERS } }
}

/// Generic camera that rasterizes the scene into a named GPU texture every
/// frame. The camera position and orientation come from the owning entity's
/// world transform; local -Z is forward and local +Y is up.
///
/// Custom shaders request the output by placing `target_name` in
/// `CustomShaderMaterial::render_textures`. Slot zero is exposed at group 0,
/// binding 11, slot one at binding 12, and so on through binding 14.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderTextureCamera {
    pub target_name: String,
    pub width: u32,
    pub height: u32,
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
    pub clear_color: [f32; 4],
    pub layer_mask: u32,
    /// Lower values render first. This allows one render texture to sample a
    /// texture produced by an earlier camera without adding renderer policy.
    pub order: i32,
    pub enabled: bool,
}

impl Default for RenderTextureCamera {
    fn default() -> Self {
        Self {
            target_name: "render_view".to_string(),
            width: 512,
            height: 512,
            fov_y_radians: 60.0_f32.to_radians(),
            near: 0.05,
            far: 10_000.0,
            clear_color: [0.01, 0.01, 0.015, 1.0],
            layer_mask: ALL_RENDER_LAYERS,
            order: 0,
            enabled: true,
        }
    }
}
