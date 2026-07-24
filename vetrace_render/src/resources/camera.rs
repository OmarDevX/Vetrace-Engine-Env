use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 2.0, 6.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov_y_radians: 60.0_f32.to_radians(),
            near: 0.05,
            far: 10_000.0,
        }
    }
}
