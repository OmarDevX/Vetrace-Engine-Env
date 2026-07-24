use super::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum PrimitiveShape {
    Cube,
    Sphere,
    Capsule,
    Plane,
    Quad,
}

impl Default for PrimitiveShape {
    fn default() -> Self { Self::Cube }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Shape {
    pub primitive: PrimitiveShape,
    pub size: Vec3,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sprite3D {
    pub texture: Option<String>,
    pub size: Vec2,
    pub billboard: bool,
}

/// Generic screen-space rectangle primitive.
///
/// This is intentionally not a crosshair/HUD component. Games and UI plugins can
/// use it for any simple overlay rectangle while the renderer stays policy-free.
/// `anchor` is normalized screen space where (0,0) is top-left and (1,1) is
/// bottom-right. `offset_px` and `size_px` are physical pixels.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScreenSpaceRect {
    pub anchor: Vec2,
    pub offset_px: Vec2,
    pub size_px: Vec2,
    pub z_order: i32,
}

impl Default for ScreenSpaceRect {
    fn default() -> Self {
        Self {
            anchor: Vec2::new(0.5, 0.5),
            offset_px: Vec2::ZERO,
            size_px: Vec2::new(16.0, 2.0),
            z_order: 0,
        }
    }
}
