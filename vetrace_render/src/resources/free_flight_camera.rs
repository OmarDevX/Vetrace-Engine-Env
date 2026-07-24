use glam::Vec3;
use serde::{Deserialize, Serialize};
use vetrace_core::InputState;

use super::Camera;

/// Reusable runtime-neutral free-flight controller for the renderer's active camera.
///
/// Platform backends only need to populate `InputState`; games and examples can
/// reuse this controller without depending on winit or SDL key/event types.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FreeFlightCameraController {
    pub enabled: bool,
    pub movement_speed: f32,
    pub boost_multiplier: f32,
    pub look_sensitivity: f32,
    pub wheel_speed_factor: f32,
    pub minimum_speed: f32,
    pub maximum_speed: f32,
    pub yaw: f32,
    pub pitch: f32,
    #[serde(skip)]
    initialized: bool,
}

impl Default for FreeFlightCameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            movement_speed: 4.5,
            boost_multiplier: 4.0,
            look_sensitivity: 0.0022,
            wheel_speed_factor: 1.15,
            minimum_speed: 0.25,
            maximum_speed: 100.0,
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            initialized: false,
        }
    }
}

impl FreeFlightCameraController {
    /// Returns a controller configured with the requested base movement speed.
    /// Internal runtime state remains encapsulated and is initialized lazily
    /// from the active camera on the first update.
    pub fn with_movement_speed(mut self, movement_speed: f32) -> Self {
        self.movement_speed = movement_speed.max(0.0);
        self
    }

    /// Rebuilds yaw/pitch from the camera's current target direction.
    pub fn synchronize_from_camera(&mut self, camera: &Camera) {
        let forward = (camera.target - camera.position).normalize_or_zero();
        if forward.length_squared() > 1.0e-8 {
            self.pitch = forward.y.clamp(-1.0, 1.0).asin();
            self.yaw = forward.z.atan2(forward.x);
        }
        self.initialized = true;
    }

    /// Applies mouse-look, WASD, vertical movement, boost, and wheel speed input.
    pub fn update(&mut self, input: &InputState, camera: &mut Camera, delta_seconds: f32) {
        if !self.enabled {
            return;
        }
        if !self.initialized {
            self.synchronize_from_camera(camera);
        }

        let (mouse_dx, mouse_dy) = input.mouse_delta();
        self.yaw += mouse_dx * self.look_sensitivity;
        self.pitch = (self.pitch - mouse_dy * self.look_sensitivity)
            .clamp(-1.553_343, 1.553_343);

        let wheel = input.mouse_wheel_delta().1;
        if wheel.abs() > f32::EPSILON {
            let factor = self.wheel_speed_factor.max(1.001).powf(wheel);
            let minimum_speed = self.minimum_speed.max(0.001);
            let maximum_speed = self.maximum_speed.max(minimum_speed);
            self.movement_speed = (self.movement_speed * factor)
                .clamp(minimum_speed, maximum_speed);
        }

        let forward = Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();

        let mut movement = Vec3::ZERO;
        if input.is_key_down("W") { movement += forward; }
        if input.is_key_down("S") { movement -= forward; }
        if input.is_key_down("D") { movement += right; }
        if input.is_key_down("A") { movement -= right; }
        if input.is_key_down("E") || input.is_key_down("Space") { movement += Vec3::Y; }
        if input.is_key_down("Q") || input.is_key_down("Control") { movement -= Vec3::Y; }

        let boost = if input.is_key_down("Shift") {
            self.boost_multiplier.max(1.0)
        } else {
            1.0
        };
        camera.position += movement.normalize_or_zero()
            * self.movement_speed.max(0.0)
            * boost
            * delta_seconds.max(0.0);
        camera.target = camera.position + forward;
        camera.up = Vec3::Y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_speed_builder_preserves_private_runtime_state() {
        let controller = FreeFlightCameraController::default().with_movement_speed(9.0);
        assert_eq!(controller.movement_speed, 9.0);
        assert!(!controller.initialized);
    }

    #[test]
    fn synchronizes_from_camera_direction() {
        let camera = Camera {
            position: Vec3::ZERO,
            target: Vec3::NEG_Z,
            ..Camera::default()
        };
        let mut controller = FreeFlightCameraController::default();
        controller.synchronize_from_camera(&camera);
        assert!((controller.yaw + std::f32::consts::FRAC_PI_2).abs() < 1.0e-5);
        assert!(controller.pitch.abs() < 1.0e-5);
    }
}
