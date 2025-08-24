use glam::{Quat, Vec2, Vec3};
use sdl2::{keyboard::Keycode, mouse::MouseButton};

use crate::ecs::behaviour::Behaviour;
use crate::engine::engine::Engine;
use crate::math::array_to_vec3;

pub struct SelectionSystem {
    prev_down: bool,
}

impl SelectionSystem {
    pub fn new() -> Self {
        Self { prev_down: false }
    }

    fn screen_ray(engine: &Engine, x: f32, y: f32) -> (Vec3, Vec3) {
        let (w, h) = engine.window.get_size();
        let aspect = w as f32 / h as f32;
        let mut uv = Vec2::new((x + 0.5) / w as f32, (y + 0.5) / h as f32);
        // SDL provides the mouse Y position with the origin at the top of the
        // window while the camera calculations expect the origin at the bottom.
        // Flip the Y coordinate so picking uses the correct orientation.
        uv.y = 1.0 - uv.y;
        uv = uv * 2.0 - Vec2::new(1.0, 1.0);
        uv.x *= aspect;
        let cam = engine.active_camera_info();
        let front = cam.orientation * Vec3::X;
        let up = cam.orientation * Vec3::Y;
        let right = cam.orientation * Vec3::Z;
        let scale = (cam.fov * 0.5).tan();
        let dir = (front + uv.x * scale * right + uv.y * scale * up).normalize();
        (cam.position, dir)
    }

    fn sphere_intersect(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
        let oc = origin - center;
        let b = oc.dot(dir);
        let c = oc.length_squared() - radius * radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let s = disc.sqrt();
        let t1 = -b - s;
        if t1 > 0.0 {
            Some(t1)
        } else {
            let t2 = -b + s;
            if t2 > 0.0 {
                Some(t2)
            } else {
                None
            }
        }
    }

    fn cube_intersect(origin: Vec3, dir: Vec3, pos: Vec3, size: Vec3, orient: Quat) -> Option<f32> {
        let inv_q = orient.conjugate();
        let local_origin = inv_q * (origin - pos);
        let local_dir = inv_q * dir;
        let inv = Vec3::new(1.0 / local_dir.x, 1.0 / local_dir.y, 1.0 / local_dir.z);
        let min_b = -size * 0.5;
        let max_b = size * 0.5;
        let t0 = (min_b - local_origin) * inv;
        let t1 = (max_b - local_origin) * inv;
        let tmin = f32::max(f32::max(t0.x.min(t1.x), t0.y.min(t1.y)), t0.z.min(t1.z));
        let tmax = f32::min(f32::min(t0.x.max(t1.x), t0.y.max(t1.y)), t0.z.max(t1.z));
        if tmax >= tmin.max(0.0) {
            if tmin > 0.0 {
                Some(tmin)
            } else if tmax > 0.0 {
                Some(tmax)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn triangle_intersect(origin: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let h = dir.cross(e2);
        let a = e1.dot(h);
        if a.abs() < 1e-8 {
            return None;
        }
        let f = 1.0 / a;
        let s = origin - v0;
        let u = f * s.dot(h);
        if u < 0.0 || u > 1.0 {
            return None;
        }
        let q = s.cross(e1);
        let v = f * dir.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * e2.dot(q);
        if t > 0.0 {
            Some(t)
        } else {
            None
        }
    }

    fn mesh_intersect(
        engine: &Engine,
        origin: Vec3,
        dir: Vec3,
        pos: Vec3,
        orient: Quat,
        obj: &crate::scene::object::Object,
    ) -> Option<f32> {
        let inv_q = orient.conjugate();
        let local_origin = inv_q * (origin - pos);
        let local_dir = inv_q * dir;
        let start = obj.triangle_start_idx;
        let end = start + obj.triangle_count;
        let mut best = f32::MAX;
        for tri in &engine.scene.triangles[start..end] {
            let v0 = Vec3::from_array(tri.v0);
            let v1 = v0 + Vec3::from_array(tri.e1);
            let v2 = v0 + Vec3::from_array(tri.e2);
            if let Some(t) = Self::triangle_intersect(local_origin, local_dir, v0, v1, v2) {
                if t < best {
                    best = t;
                }
            }
        }
        if best < f32::MAX {
            Some(best)
        } else {
            None
        }
    }

    fn pick(engine: &Engine, mx: i32, my: i32) -> Option<crate::ecs::Entity> {
        let (origin, dir) = Self::screen_ray(engine, mx as f32, my as f32);
        let mut best_t = f32::MAX;
        let mut best_idx = None;
        for (i, obj) in engine.scene.objects.iter().enumerate() {
            let pos = array_to_vec3(obj.position);
            let orient = Quat::from_xyzw(
                obj.orientation[0],
                obj.orientation[1],
                obj.orientation[2],
                obj.orientation[3],
            );
            let size = array_to_vec3(obj.size) * array_to_vec3(obj.scale);
            let t = if obj.is_mesh {
                Self::mesh_intersect(engine, origin, dir, pos, orient, obj)
            } else if obj.is_cube {
                Self::cube_intersect(origin, dir, pos, size, orient)
            } else {
                let s = obj.scale[0].max(obj.scale[1]).max(obj.scale[2]);
                Self::sphere_intersect(origin, dir, pos, obj.radius * s)
            };
            if let Some(dist) = t {
                if dist < best_t {
                    best_t = dist;
                    best_idx = Some(i);
                }
            }
        }
        if let Some(idx) = best_idx {
            engine.core.find_entity_by_object_id(idx as u32)
        } else {
            None
        }
    }
}

impl Behaviour for SelectionSystem {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        // Note: Selection functionality has been moved to the vetrace_editor crate
        // This is kept for compatibility but doesn't perform actual selection

        let down = engine.input.is_mouse_button_down(sdl2::mouse::MouseButton::Left);
        if down && !self.prev_down {
            if engine.egui_ctx.wants_pointer_input() {
                self.prev_down = down;
                return;
            }
            let (mx, my) = engine.input.mouse_position();
            if let Some(_ent) = Self::pick(engine, mx, my) {
                // Selection logic moved to editor plugin
                println!("Entity picked at ({}, {}) - use editor plugin for selection", mx, my);
            }
        }
        self.prev_down = down;
    }
}