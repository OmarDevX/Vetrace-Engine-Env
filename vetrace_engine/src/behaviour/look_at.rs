use crate::math::*;
use ahash::AHashMap;
use glam::{EulerRot, Quat, Vec3};

use crate::{
    Behaviour,
    components::components::{LookAt, Metadata, Transform},
    engine::engine::Engine,
};

pub struct LookAtBehaviour;

impl Behaviour for LookAtBehaviour {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        let mut targets: AHashMap<String, [f32; 3]> = AHashMap::new();
        for ent in engine.world.entities().to_vec() {
            if let Some(meta) = engine.world.get::<Metadata>(ent) {
                if let Some(g) = engine
                    .world
                    .get::<crate::components::components::GlobalTransform>(ent)
                {
                    targets.insert(meta.name.clone(), g.position);
                } else if let Some(t) = engine.world.get::<Transform>(ent) {
                    targets.insert(meta.name.clone(), t.position);
                }
            }
        }

        let cam = engine.active_camera_info();
        let query = engine.world.query2::<Transform, LookAt>();
        let mut updates: Vec<(crate::ecs::Entity, [f32; 4])> = Vec::new();
        for (e, transform, look) in query {
            let self_pos = if let Some(g) =
                engine
                    .world
                    .get::<crate::components::components::GlobalTransform>(e)
            {
                g.position
            } else {
                transform.position
            };

            let target_pos = if look.target.to_lowercase() == "mouse" {
                if engine.egui_ctx.wants_pointer_input() {
                    None
                } else {
                    let (mx, my) = engine.input.mouse_position();
                    if engine.is_2d {
                        let (w, h) = engine.window.get_size();
                        let screen_center = (w as f32 * 0.5, h as f32 * 0.5);
                        let scale = h as f32 / (cam.fov * 10.0);
                        Some([
                            (mx as f32 - screen_center.0) / scale + cam.position.x,
                            (screen_center.1 - my as f32) / scale + cam.position.y,
                            0.0,
                        ])
                    } else {
                        let (w, h) = engine.window.get_size();
                        let mut uv_x = ((mx as f32 + 0.5) / w as f32) * 2.0 - 1.0;
                        let uv_y = 1.0 - ((my as f32 + 0.5) / h as f32) * 2.0;
                        uv_x *= w as f32 / h as f32;
                        let front = cam.orientation * Vec3::X;
                        let up = cam.orientation * Vec3::Y;
                        let right = cam.orientation * Vec3::Z;
                        let dir = (front + right * uv_x + up * uv_y).normalize();

                        if dir.y.abs() > 1e-6 {
                            let t = (self_pos[1] - cam.position.y) / dir.y;
                            let world = cam.position + dir * t;
                            Some([world.x, world.y, world.z])
                        } else {
                            None
                        }
                    }
                }
            } else {
                targets.get(&look.target).copied()
            };

            let target_pos = match target_pos {
                Some(p) => p,
                None => continue,
            };

            let dir = Vec3::new(
                target_pos[0] - self_pos[0],
                target_pos[1] - self_pos[1],
                target_pos[2] - self_pos[2],
            );
            if dir.length() == 0.0 {
                continue;
            }

            let new_ori = if engine.is_2d {
                if look.rotate_z {
                    let angle = dir.y.atan2(dir.x);
                    let q = Quat::from_axis_angle(Vec3::Z, angle);
                    [q.x, q.y, q.z, q.w]
                } else {
                    transform.orientation
                }
            } else {
                let quat = Quat::from_rotation_arc(Vec3::Z, dir.normalize());
                let (mut roll, mut pitch, mut yaw) = quat.to_euler(EulerRot::XYZ);

                if !look.rotate_x {
                    roll = 0.0;
                }
                if !look.rotate_y {
                    pitch = 0.0;
                }
                if !look.rotate_z {
                    yaw = 0.0;
                }

                let final_q = Quat::from_euler(EulerRot::XYZ, roll, pitch, yaw);
                [final_q.x, final_q.y, final_q.z, final_q.w]
            };

            updates.push((e, new_ori));
        }

        for (e, ori) in updates {
            if let Some(t) = engine.world.get_mut::<Transform>(e) {
                t.orientation = ori;
            }
        }
    }
}
