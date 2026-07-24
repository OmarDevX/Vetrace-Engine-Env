use glam::{Mat2, Vec2};
use serde::{Deserialize, Serialize};

/// Orthographic, Y-up camera used by the optional 2D canvas renderer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera2D {
    pub position: Vec2,
    pub rotation: f32,
    pub zoom: f32,
    pub pixels_per_unit: f32,
    pub pixel_snap: bool,
    /// Physical-pixel origin of the canvas viewport inside the render surface.
    /// Runtime games normally leave this at zero; Studio updates it to the
    /// unobstructed center workspace between dock panels.
    #[serde(default)]
    pub viewport_origin_px: Vec2,
    /// Optional physical-pixel viewport size. `None` uses the complete surface.
    #[serde(default)]
    pub viewport_size_px: Option<Vec2>,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            rotation: 0.0,
            zoom: 1.0,
            pixels_per_unit: 100.0,
            pixel_snap: false,
            viewport_origin_px: Vec2::ZERO,
            viewport_size_px: None,
        }
    }
}

impl Camera2D {
    pub fn pixels_per_world_unit(&self) -> f32 {
        let zoom = if self.zoom.is_finite() { self.zoom.max(0.0001) } else { 1.0 };
        let ppu = if self.pixels_per_unit.is_finite() {
            self.pixels_per_unit.max(0.0001)
        } else {
            100.0
        };
        ppu * zoom
    }

    /// Returns `(origin, size)` in physical surface pixels.
    pub fn viewport_rect_px(&self, surface_size_px: Vec2) -> (Vec2, Vec2) {
        let surface = surface_size_px.max(Vec2::ONE);
        let mut origin = if self.viewport_origin_px.is_finite() {
            self.viewport_origin_px.max(Vec2::ZERO)
        } else {
            Vec2::ZERO
        };
        origin = origin.min(surface - Vec2::ONE);
        let requested = self
            .viewport_size_px
            .filter(|size| size.is_finite() && size.x > 0.0 && size.y > 0.0)
            .unwrap_or(surface);
        let size = requested.min(surface - origin).max(Vec2::ONE);
        (origin, size)
    }

    pub fn set_viewport_px(&mut self, origin: Vec2, size: Vec2) {
        self.viewport_origin_px = origin;
        self.viewport_size_px = Some(size);
    }

    pub fn clear_viewport(&mut self) {
        self.viewport_origin_px = Vec2::ZERO;
        self.viewport_size_px = None;
    }

    pub fn world_to_screen(&self, world: Vec2, surface_size_px: Vec2) -> Vec2 {
        let (origin, viewport) = self.viewport_rect_px(surface_size_px);
        let rotation = Mat2::from_angle(-self.rotation);
        let local = rotation * (world - self.position);
        origin
            + viewport * 0.5
            + Vec2::new(
                local.x * self.pixels_per_world_unit(),
                -local.y * self.pixels_per_world_unit(),
            )
    }

    pub fn screen_to_world(&self, screen_px: Vec2, surface_size_px: Vec2) -> Vec2 {
        let (origin, viewport) = self.viewport_rect_px(surface_size_px);
        let local = Vec2::new(
            screen_px.x - origin.x - viewport.x * 0.5,
            origin.y + viewport.y * 0.5 - screen_px.y,
        ) / self.pixels_per_world_unit();
        self.position + Mat2::from_angle(self.rotation) * local
    }

    pub fn visible_half_extents(&self, surface_size_px: Vec2) -> Vec2 {
        let (_, viewport) = self.viewport_rect_px(surface_size_px);
        viewport * 0.5 / self.pixels_per_world_unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_screen_conversion_round_trips_with_rotation_and_zoom() {
        let camera = Camera2D {
            position: Vec2::new(3.0, -2.0),
            rotation: 0.37,
            zoom: 2.25,
            pixels_per_unit: 64.0,
            pixel_snap: false,
            viewport_origin_px: Vec2::new(180.0, 72.0),
            viewport_size_px: Some(Vec2::new(900.0, 600.0)),
        };
        let surface = Vec2::new(1280.0, 720.0);
        let world = Vec2::new(-1.25, 8.5);
        let screen = camera.world_to_screen(world, surface);
        let restored = camera.screen_to_world(screen, surface);
        assert!((restored - world).length() < 0.0001);
    }

    #[test]
    fn viewport_is_clamped_inside_surface() {
        let camera = Camera2D {
            viewport_origin_px: Vec2::new(1200.0, 700.0),
            viewport_size_px: Some(Vec2::new(500.0, 500.0)),
            ..Camera2D::default()
        };
        let (origin, size) = camera.viewport_rect_px(Vec2::new(1280.0, 720.0));
        assert_eq!(origin, Vec2::new(1200.0, 700.0));
        assert_eq!(size, Vec2::new(80.0, 20.0));
    }
}
