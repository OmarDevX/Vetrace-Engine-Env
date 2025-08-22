use crate::components::components::{FreeFlightControls, Transform, LookAt};
use crate::ecs::World;
use crate::input::Input;
use glam::{Quat, Vec3};
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;

pub struct FreeFlightState {
    pub last_mouse_x: i32,
    pub last_mouse_y: i32,
    pub first_mouse: bool,
}

impl FreeFlightState {
    pub fn new() -> Self {
        Self {
            last_mouse_x: 0,
            last_mouse_y: 0,
            first_mouse: true,
        }
    }

    pub fn update(&mut self, world: &mut World, input: &Input, delta_time: f32) {
    // First collect all entities with LookAt
    let lookat_entities: std::collections::HashSet<_> = world
        .query::<LookAt>()
        .iter()
        .map(|(entity, _)| *entity)
        .collect();

    // Then do mutable query
    let mut query = world.query2_mut::<Transform, FreeFlightControls>();
    for (entity, transform, controls) in query.iter_mut() {
        let has_lookat = lookat_entities.contains(entity);
        
        // Build orientation from yaw (around Y axis) and pitch (around right axis)
        let mut q = Quat::from_rotation_y(-controls.yaw.to_radians())
            * Quat::from_rotation_z(-controls.pitch.to_radians());
            
        if has_lookat {
            // When LookAt is present...
            q = Quat::from_xyzw(
                transform.orientation[0],
                transform.orientation[1],
                transform.orientation[2],
                transform.orientation[3],
            );
        }
            let front = q * Vec3::X;
            let up = q * Vec3::Y;
            let right = q * Vec3::Z;
            let mut moving = false;
            if input.is_mouse_button_down(MouseButton::Right) {
                if input.is_key_down(Keycode::W) {
                    controls.velocity[0] += front.x * controls.acceleration * delta_time;
                    controls.velocity[1] += front.y * controls.acceleration * delta_time;
                    controls.velocity[2] += front.z * controls.acceleration * delta_time;
                    moving = true;
                }
                if input.is_key_down(Keycode::S) {
                    controls.velocity[0] -= front.x * controls.acceleration * delta_time;
                    controls.velocity[1] -= front.y * controls.acceleration * delta_time;
                    controls.velocity[2] -= front.z * controls.acceleration * delta_time;
                    moving = true;
                }
                if input.is_key_down(Keycode::A) {
                    controls.velocity[0] -= right.x * controls.acceleration * delta_time;
                    controls.velocity[1] -= right.y * controls.acceleration * delta_time;
                    controls.velocity[2] -= right.z * controls.acceleration * delta_time;
                    moving = true;
                }
                if input.is_key_down(Keycode::D) {
                    controls.velocity[0] += right.x * controls.acceleration * delta_time;
                    controls.velocity[1] += right.y * controls.acceleration * delta_time;
                    controls.velocity[2] += right.z * controls.acceleration * delta_time;
                    moving = true;
                }
                if input.is_key_down(Keycode::E) {
                    controls.velocity[0] += up.x * controls.acceleration * delta_time;
                    controls.velocity[1] += up.y * controls.acceleration * delta_time;
                    controls.velocity[2] += up.z * controls.acceleration * delta_time;
                    moving = true;
                }
                if input.is_key_down(Keycode::Q) {
                    controls.velocity[0] -= up.x * controls.acceleration * delta_time;
                    controls.velocity[1] -= up.y * controls.acceleration * delta_time;
                    controls.velocity[2] -= up.z * controls.acceleration * delta_time;
                    moving = true;
                }
                if input.is_key_down(Keycode::Space) {
                    controls.velocity = [0.0, 0.0, 0.0];
                }
            }

            if input.is_mouse_button_down(MouseButton::Right) {
                let (mouse_x, mouse_y) = input.mouse_position();
                if self.first_mouse {
                    self.last_mouse_x = mouse_x;
                    self.last_mouse_y = mouse_y;
                    self.first_mouse = false;
                }
                let xoffset = (mouse_x - self.last_mouse_x) as f32;
                let yoffset = (mouse_y - self.last_mouse_y) as f32; // Fixed: inverted Y axis
                self.last_mouse_x = mouse_x;
                self.last_mouse_y = mouse_y;
                controls.yaw_velocity += xoffset * controls.sensitivity;
                controls.pitch_velocity += yoffset * controls.sensitivity;
            } else {
                self.first_mouse = true;
            }

            controls.yaw += controls.yaw_velocity * delta_time;
            controls.pitch += controls.pitch_velocity * delta_time;

            controls.yaw_velocity *= controls.angular_friction.powf(delta_time);
            controls.pitch_velocity *= controls.angular_friction.powf(delta_time);

            if controls.pitch > 89.0 {
                controls.pitch = 89.0;
                controls.pitch_velocity = 0.0;
            }
            if controls.pitch < -89.0 {
                controls.pitch = -89.0;
                controls.pitch_velocity = 0.0;
            }

            if moving {
                let factor = (1.0 - controls.friction * delta_time).max(0.0);
                controls.velocity[0] *= factor;
                controls.velocity[1] *= factor;
                controls.velocity[2] *= factor;
            } else {
                let speed = (controls.velocity[0] * controls.velocity[0]
                    + controls.velocity[1] * controls.velocity[1]
                    + controls.velocity[2] * controls.velocity[2])
                    .sqrt();
                if speed > 0.0 {
                    let decel = controls.deceleration * delta_time;
                    let new_speed = (speed - decel).max(0.0);
                    if new_speed == 0.0 {
                        controls.velocity = [0.0, 0.0, 0.0];
                    } else {
                        controls.velocity[0] = controls.velocity[0] / speed * new_speed;
                        controls.velocity[1] = controls.velocity[1] / speed * new_speed;
                        controls.velocity[2] = controls.velocity[2] / speed * new_speed;
                    }
                }
            }

            let speed = (controls.velocity[0] * controls.velocity[0]
                + controls.velocity[1] * controls.velocity[1]
                + controls.velocity[2] * controls.velocity[2])
                .sqrt();
            if speed < 0.00005 {
                controls.velocity = [0.0, 0.0, 0.0];
            }
            if speed > controls.speed {
                controls.velocity[0] = controls.velocity[0] / speed * controls.speed;
                controls.velocity[1] = controls.velocity[1] / speed * controls.speed;
                controls.velocity[2] = controls.velocity[2] / speed * controls.speed;
            }

            transform.position[0] += controls.velocity[0] * delta_time;
            transform.position[1] += controls.velocity[1] * delta_time;
            transform.position[2] += controls.velocity[2] * delta_time;

            if !has_lookat {
                        transform.orientation = [q.x, q.y, q.z, q.w];
            }
        }
    }
}