use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Optional visual customization shared by panels, buttons, text editors and
/// color rectangles.  Keeping this separate from the widget data lets games
/// build a consistent theme without renderer-specific components.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct UIVisualStyle {
    pub corner_radius: f32,
    pub border_width: f32,
    pub border_color: Vec3,
    pub border_alpha: f32,
    pub text_color: Vec3,
    pub text_alpha: f32,
    pub font_size: f32,
    /// Amount added toward white while hovered (0 = no change, 1 = white).
    pub hover_brightness: f32,
    /// Amount added toward black while pressed (0 = no change, 1 = black).
    pub pressed_darkness: f32,
    pub shadow_color: Vec3,
    pub shadow_alpha: f32,
    pub shadow_offset: Vec2,
}

impl Default for UIVisualStyle {
    fn default() -> Self {
        Self {
            corner_radius: 8.0,
            border_width: 0.0,
            border_color: Vec3::ONE,
            border_alpha: 0.0,
            text_color: Vec3::ONE,
            text_alpha: 1.0,
            font_size: 17.0,
            hover_brightness: 0.12,
            pressed_darkness: 0.20,
            shadow_color: Vec3::ZERO,
            shadow_alpha: 0.0,
            shadow_offset: Vec2::new(0.0, 4.0),
        }
    }
}

impl UIVisualStyle {
    pub fn rounded(radius: f32) -> Self {
        Self { corner_radius: radius.max(0.0), ..Self::default() }
    }

    pub fn with_border(mut self, width: f32, color: Vec3, alpha: f32) -> Self {
        self.border_width = width.max(0.0);
        self.border_color = color;
        self.border_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    pub fn with_shadow(mut self, offset: Vec2, color: Vec3, alpha: f32) -> Self {
        self.shadow_offset = offset;
        self.shadow_color = color;
        self.shadow_alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

/// Generic pointer result used by game-side UI controllers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UIInteraction {
    pub hovered: bool,
    pub pressed: bool,
    pub clicked: bool,
}

/// Converts normalized anchor + pixel offset UI placement into a pixel rect.
pub fn screen_rect_bounds(
    viewport_px: Vec2,
    anchor: Vec2,
    offset_px: Vec2,
    size_px: Vec2,
) -> (Vec2, Vec2) {
    let center = anchor * viewport_px.max(Vec2::ONE) + offset_px;
    let half = size_px.max(Vec2::ZERO) * 0.5;
    (center - half, center + half)
}

pub fn screen_rect_contains(
    viewport_px: Vec2,
    anchor: Vec2,
    offset_px: Vec2,
    size_px: Vec2,
    point_px: Vec2,
) -> bool {
    let (min, max) = screen_rect_bounds(viewport_px, anchor, offset_px, size_px);
    point_px.x >= min.x && point_px.x <= max.x && point_px.y >= min.y && point_px.y <= max.y
}

pub fn pointer_interaction(
    viewport_px: Vec2,
    anchor: Vec2,
    offset_px: Vec2,
    size_px: Vec2,
    point_px: Vec2,
    pointer_down: bool,
    pointer_released: bool,
) -> UIInteraction {
    let hovered = screen_rect_contains(viewport_px, anchor, offset_px, size_px, point_px);
    UIInteraction {
        hovered,
        pressed: hovered && pointer_down,
        clicked: hovered && pointer_released,
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Default for Anchor {
    fn default() -> Self { Self::TopLeft }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl Default for TextAlign {
    fn default() -> Self { Self::Left }
}


/// Marks any supported `vetrace_ui` widget as anchored to a 3D world transform.
///
/// This is a placement component, not a label component. Attach it to an entity
/// that also has `Transform`/`GlobalTransform` plus one of the normal UI
/// components (`UILabel`, `UIPanel`, `UIButton`, `UITextEditor`, `UIList`, or
/// `ColorRect`). Ownership/following stays in the core scene hierarchy: parent
/// the UI entity to any world entity and give it a local transform offset.
///
/// Current WGPU support renders world UI as an egui overlay projected from the
/// entity's world position. That keeps text/buttons readable and clickable-style
/// state display stable instead of perspective-warping it like a mesh.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UIWorldSpace {
    /// Extra 2D pixel nudge after projection. Useful for nameplates and prompts.
    #[serde(default)]
    pub screen_offset_px: Vec2,
    /// Optional pixel size override for widgets that do not carry their own size.
    /// `Vec2::ZERO` means the renderer should use the widget's natural/default size.
    #[serde(default)]
    pub size_px: Vec2,
    /// Hide the widget beyond this camera distance. Values <= 0 disable the limit.
    #[serde(default = "default_world_ui_max_distance")]
    pub max_distance: f32,
    /// Draw order among projected world UI widgets. Higher values draw later.
    #[serde(default)]
    pub z_order: i32,
    /// Pivot used when placing the projected UI rectangle around the projected point.
    #[serde(default = "default_world_ui_anchor")]
    pub anchor: Anchor,
    /// Optional rounded background color used by label-style world widgets.
    #[serde(default)]
    pub background: Vec3,
    #[serde(default = "default_world_ui_background_alpha")]
    pub background_alpha: f32,
    #[serde(default = "default_world_ui_padding_px")]
    pub padding_px: Vec2,
    #[serde(default = "default_true")]
    pub visible: bool,
}

fn default_world_ui_max_distance() -> f32 { 80.0 }
fn default_world_ui_anchor() -> Anchor { Anchor::BottomCenter }
fn default_world_ui_background_alpha() -> f32 { 0.55 }
fn default_world_ui_padding_px() -> Vec2 { Vec2::new(7.0, 3.0) }
fn default_true() -> bool { true }

impl Default for UIWorldSpace {
    fn default() -> Self {
        Self {
            screen_offset_px: Vec2::ZERO,
            size_px: Vec2::ZERO,
            max_distance: default_world_ui_max_distance(),
            z_order: 0,
            anchor: default_world_ui_anchor(),
            background: Vec3::ZERO,
            background_alpha: default_world_ui_background_alpha(),
            padding_px: default_world_ui_padding_px(),
            visible: true,
        }
    }
}

/// Marker for UI entities positioned in screen space rather than world space.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct UIScreenSpace;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UILabel {
    pub text: String,
    pub font_size: f32,
    pub color: Vec3,
    pub anchor: Anchor,
    pub align: TextAlign,
}

impl Default for UILabel {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_size: 18.0,
            color: Vec3::ONE,
            anchor: Anchor::TopLeft,
            align: TextAlign::Left,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UIPanel {
    pub size: Vec2,
    pub background: Vec3,
    pub alpha: f32,
    pub anchor: Anchor,
}

impl Default for UIPanel {
    fn default() -> Self {
        Self {
            size: Vec2::new(160.0, 80.0),
            background: Vec3::splat(0.1),
            alpha: 0.85,
            anchor: Anchor::TopLeft,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UIButton {
    pub text: String,
    pub size: Vec2,
    pub hovered: bool,
    pub pressed: bool,
    pub enabled: bool,
}

impl Default for UIButton {
    fn default() -> Self {
        Self {
            text: String::new(),
            size: Vec2::new(120.0, 32.0),
            hovered: false,
            pressed: false,
            enabled: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UITextEditor {
    pub text: String,
    pub placeholder: String,
    pub focused: bool,
    pub multiline: bool,
}

impl Default for UITextEditor {
    fn default() -> Self {
        Self {
            text: String::new(),
            placeholder: String::new(),
            focused: false,
            multiline: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UIList {
    pub items: Vec<String>,
    pub selected: Option<usize>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LayoutDirection {
    Horizontal,
    Vertical,
}

impl Default for LayoutDirection {
    fn default() -> Self { Self::Vertical }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UILayout {
    pub direction: LayoutDirection,
    pub spacing: f32,
    pub padding: f32,
}

impl Default for UILayout {
    fn default() -> Self {
        Self { direction: LayoutDirection::Vertical, spacing: 4.0, padding: 0.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorRect {
    pub size: Vec2,
    pub color: Vec3,
    pub alpha: f32,
}

impl Default for ColorRect {
    fn default() -> Self {
        Self { size: Vec2::new(100.0, 100.0), color: Vec3::ONE, alpha: 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchored_hit_testing_uses_viewport_and_offset() {
        let viewport = Vec2::new(1280.0, 720.0);
        assert!(screen_rect_contains(
            viewport,
            Vec2::new(1.0, 1.0),
            Vec2::new(-100.0, -50.0),
            Vec2::new(120.0, 60.0),
            Vec2::new(1180.0, 670.0),
        ));
        assert!(!screen_rect_contains(
            viewport,
            Vec2::new(1.0, 1.0),
            Vec2::new(-100.0, -50.0),
            Vec2::new(120.0, 60.0),
            Vec2::new(1100.0, 670.0),
        ));
    }

    #[test]
    fn click_requires_release_inside() {
        let interaction = pointer_interaction(
            Vec2::new(800.0, 600.0),
            Vec2::splat(0.5),
            Vec2::ZERO,
            Vec2::new(200.0, 80.0),
            Vec2::new(400.0, 300.0),
            false,
            true,
        );
        assert_eq!(interaction, UIInteraction { hovered: true, pressed: false, clicked: true });
    }
}
