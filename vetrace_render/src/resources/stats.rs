use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RenderStats {
    pub frames_rendered: u64,
    pub visible_objects: usize,
    #[cfg(feature = "render_2d")]
    pub visible_sprites_2d: usize,
    pub directional_lights: usize,
    pub point_lights: usize,
    pub spot_lights: usize,
    pub has_atmosphere: bool,
    pub has_fog: bool,
}
