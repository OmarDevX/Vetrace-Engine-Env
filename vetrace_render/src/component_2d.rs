use glam::{Quat, Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use vetrace_core::Transform;

use crate::components::TextureHandle;

/// Blend policy used by a 2D canvas item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum BlendMode2D {
    #[default]
    Alpha,
    Additive,
    Multiply,
}

/// Texture filtering used by a 2D sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, vetrace_core::VetraceEnum)]
pub enum TextureFilter2D {
    Nearest,
    #[default]
    Linear,
}

/// Common visibility and deterministic canvas ordering for 2D renderables.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanvasItem2D {
    pub visible: bool,
    pub canvas_layer: i32,
    pub z_index: i32,
    pub blend_mode: BlendMode2D,
}

impl Default for CanvasItem2D {
    fn default() -> Self {
        Self {
            visible: true,
            canvas_layer: 0,
            z_index: 0,
            blend_mode: BlendMode2D::Alpha,
        }
    }
}

/// Pixel-space source rectangle inside a sprite texture.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Rect2D {
    pub min: Vec2,
    pub size: Vec2,
}

/// World-space textured quad rendered by the dedicated 2D canvas pass.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sprite2D {
    /// Runtime texture handle. Scene loading reconstructs this from
    /// `texture_path`, so portable scenes do not depend on process-local IDs.
    #[serde(skip)]
    pub texture: Option<TextureHandle>,
    /// Portable project/scene-relative texture path.
    pub texture_path: Option<String>,
    /// Sprite size in world units before applying `Transform::scale`.
    pub size: Vec2,
    /// Normalized pivot where (0,0) is bottom-left and (1,1) is top-right.
    pub pivot: Vec2,
    /// Optional pixel-space atlas/source rectangle. `None` samples the full image.
    pub source_rect_px: Option<Rect2D>,
    pub tint: Vec4,
    pub flip_x: bool,
    pub flip_y: bool,
    pub filter: TextureFilter2D,
    pub pixel_snap: bool,
    /// Samples with alpha lower than this value are discarded.
    pub alpha_cutoff: f32,
}

impl Default for Sprite2D {
    fn default() -> Self {
        Self {
            texture: None,
            texture_path: None,
            size: Vec2::ONE,
            pivot: Vec2::splat(0.5),
            source_rect_px: None,
            tint: Vec4::ONE,
            flip_x: false,
            flip_y: false,
            filter: TextureFilter2D::Linear,
            pixel_snap: false,
            alpha_cutoff: 0.0,
        }
    }
}

/// Ergonomic 2D accessors over Vetrace's single authoritative `Transform`.
pub trait Transform2DExt {
    fn position_2d(&self) -> Vec2;
    fn set_position_2d(&mut self, position: Vec2);
    fn rotation_2d(&self) -> f32;
    fn set_rotation_2d(&mut self, radians: f32);
    fn scale_2d(&self) -> Vec2;
    fn set_scale_2d(&mut self, scale: Vec2);
}

impl Transform2DExt for Transform {
    fn position_2d(&self) -> Vec2 {
        self.translation.truncate()
    }

    fn set_position_2d(&mut self, position: Vec2) {
        self.translation.x = position.x;
        self.translation.y = position.y;
    }

    fn rotation_2d(&self) -> f32 {
        let (_, _, z) = self.rotation.to_euler(glam::EulerRot::XYZ);
        z
    }

    fn set_rotation_2d(&mut self, radians: f32) {
        self.rotation = Quat::from_rotation_z(radians);
    }

    fn scale_2d(&self) -> Vec2 {
        self.scale.truncate()
    }

    fn set_scale_2d(&mut self, scale: Vec2) {
        self.scale = Vec3::new(scale.x, scale.y, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_texture_handle_is_not_serialized() {
        let sprite = Sprite2D {
            texture: Some(TextureHandle(42)),
            texture_path: Some("assets/textures/player.png".to_owned()),
            ..Sprite2D::default()
        };
        let encoded = bincode::serialize(&sprite).unwrap();
        let decoded: Sprite2D = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded.texture, None);
        assert_eq!(decoded.texture_path.as_deref(), Some("assets/textures/player.png"));
    }
}
