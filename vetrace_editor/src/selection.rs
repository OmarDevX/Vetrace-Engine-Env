//! Selection Plugin for Entity Selection
//! 
//! This module provides entity selection functionality through mouse picking.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::ecs::Entity;
use sdl2::mouse::MouseButton;
use vetrace_engine::math::array_to_vec3;
use glam::{Vec2, Vec3, Quat};

/// Selection plugin for entity picking
pub struct SelectionPlugin {
    prev_mouse_down: bool,
    initialized: bool,
}

impl SelectionPlugin {
    /// Create a new selection plugin
    pub fn new() -> Self {
        Self {
            prev_mouse_down: false,
            initialized: false,
        }
    }
    
    /// Handle selection input
    pub fn handle_selection(&mut self, engine: &mut Engine, selected_entities: &mut Vec<Entity>, wants_input: bool) {
        let down = engine.input.is_mouse_button_down(MouseButton::Left);
        
        if down && !self.prev_mouse_down {
            // Don't handle selection if UI wants input
            if wants_input {
                self.prev_mouse_down = down;
                return;
            }
            
            let (mx, my) = engine.input.mouse_position();
            if let Some(entity) = self.pick_entity(engine, mx, my) {
                // Handle multi-selection with Ctrl
                if engine.input.is_key_down(sdl2::keyboard::Keycode::LCtrl) ||
                   engine.input.is_key_down(sdl2::keyboard::Keycode::RCtrl) {
                    if let Some(index) = selected_entities.iter().position(|e| *e == entity) {
                        // Deselect if already selected
                        selected_entities.remove(index);
                    } else {
                        // Add to selection
                        selected_entities.push(entity);
                    }
                } else {
                    // Single selection
                    selected_entities.clear();
                    selected_entities.push(entity);
                }
            } else if !(engine.input.is_key_down(sdl2::keyboard::Keycode::LCtrl) ||
                       engine.input.is_key_down(sdl2::keyboard::Keycode::RCtrl)) {
                // Clear selection if not holding Ctrl and didn't hit anything
                selected_entities.clear();
            }
        }
        
        self.prev_mouse_down = down;
    }
    
    /// Pick an entity at the given screen coordinates
    fn pick_entity(&self, engine: &Engine, mx: i32, my: i32) -> Option<Entity> {
        let (origin, dir) = self.screen_ray(engine, mx as f32, my as f32);
        let mut best_t = f32::MAX;
        let mut best_entity = None;
        
        // Check all objects in the scene
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
                self.mesh_intersect(engine, origin, dir, pos, orient, obj)
            } else if obj.is_cube {
                self.cube_intersect(origin, dir, pos, size, orient)
            } else {
                let s = obj.scale[0].max(obj.scale[1]).max(obj.scale[2]);
                self.sphere_intersect(origin, dir, pos, obj.radius * s)
            };
            
            if let Some(dist) = t {
                if dist < best_t {
                    best_t = dist;
                    if let Some(entity) = engine.core.find_entity_by_object_id(i as u32) {
                        best_entity = Some(entity);
                    }
                }
            }
        }
        
        best_entity
    }
    
    /// Create a world-space ray from SDL window coordinates.
    fn screen_ray(&self, engine: &Engine, x: f32, y: f32) -> (Vec3, Vec3) {
        let (w, h) = engine.window.get_size();
        let w = w.max(1) as f32;
        let h = h.max(1) as f32;
        let aspect = w / h;

        let mut uv = Vec2::new((x + 0.5) / w, (y + 0.5) / h);
        // SDL mouse coordinates start at the top-left. The camera projection math
        // expects bottom-left NDC, so flip Y before expanding to [-1, 1].
        uv.y = 1.0 - uv.y;
        uv = uv * 2.0 - Vec2::new(1.0, 1.0);
        uv.x *= aspect;

        let cam = engine.active_camera_info();
        let front = cam.orientation * Vec3::X;
        let up = cam.orientation * Vec3::Y;
        let right = cam.orientation * Vec3::Z;
        let scale = (cam.fov * 0.5).tan();
        let direction = (front + uv.x * scale * right + uv.y * scale * up).normalize();
        (cam.position, direction)
    }
    
    /// Test ray-sphere intersection
    fn sphere_intersect(&self, origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
        let oc = origin - center;
        let a = dir.dot(dir);
        let b = 2.0 * oc.dot(dir);
        let c = oc.dot(oc) - radius * radius;
        let discriminant: f32 = b * b - 4.0 * a * c;
        
        if discriminant < 0.0 {
            None
        } else {
            let t1 = (-b - discriminant.sqrt()) / (2.0 * a);
            let t2 = (-b + discriminant.sqrt()) / (2.0 * a);
            
            if t1 > 0.0 {
                Some(t1)
            } else if t2 > 0.0 {
                Some(t2)
            } else {
                None
            }
        }
    }
    
    /// Test ray-cube intersection. Handles rotated cubes by transforming the ray
    /// into the cube's local space before doing an AABB slab test.
    fn cube_intersect(&self, origin: Vec3, dir: Vec3, center: Vec3, size: Vec3, rotation: Quat) -> Option<f32> {
        let inv_q = rotation.conjugate();
        let local_origin = inv_q * (origin - center);
        let local_dir = inv_q * dir;
        let inv_dir = Vec3::new(
            1.0 / local_dir.x,
            1.0 / local_dir.y,
            1.0 / local_dir.z,
        );
        let min = -size * 0.5;
        let max = size * 0.5;

        let t1 = (min - local_origin) * inv_dir;
        let t2 = (max - local_origin) * inv_dir;

        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let t_near = tmin.x.max(tmin.y).max(tmin.z);
        let t_far = tmax.x.min(tmax.y).min(tmax.z);

        if t_near <= t_far && t_far > 0.0 {
            if t_near > 0.0 {
                Some(t_near)
            } else {
                Some(t_far)
            }
        } else {
            None
        }
    }

    fn triangle_intersect(&self, origin: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let h = dir.cross(e2);
        let a = e1.dot(h);
        if a.abs() < 1.0e-8 {
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
        if t > 0.0 { Some(t) } else { None }
    }

    /// Test ray-mesh intersection against the scene triangle range for this object.
    fn mesh_intersect(&self, engine: &Engine, origin: Vec3, dir: Vec3, pos: Vec3, orient: Quat, obj: &vetrace_engine::scene::object::Object) -> Option<f32> {
        let start = obj.triangle_start_idx;
        let end = start.saturating_add(obj.triangle_count);
        if start >= engine.scene.triangles.len() || end > engine.scene.triangles.len() {
            let radius = obj.size[0].max(obj.size[1]).max(obj.size[2]) * 0.5;
            return self.sphere_intersect(origin, dir, pos, radius);
        }

        let inv_q = orient.conjugate();
        let local_origin = inv_q * (origin - pos);
        let local_dir = inv_q * dir;
        let mut best = f32::MAX;
        for tri in &engine.scene.triangles[start..end] {
            let v0 = Vec3::from_array(tri.v0);
            let v1 = v0 + Vec3::from_array(tri.e1);
            let v2 = v0 + Vec3::from_array(tri.e2);
            if let Some(t) = self.triangle_intersect(local_origin, local_dir, v0, v1, v2) {
                if t < best {
                    best = t;
                }
            }
        }
        if best < f32::MAX { Some(best) } else { None }
    }
}

impl Default for SelectionPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for SelectionPlugin {
    fn name(&self) -> &'static str {
        "selection"
    }
    
    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if self.initialized {
            return Ok(());
        }
        
        println!("Initializing Selection Plugin...");
        self.initialized = true;
        
        Ok(())
    }
    
    fn update(&mut self, _engine: &mut Engine, _delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Selection update is handled by the main window
        Ok(())
    }
    
    fn render(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Selection doesn't have its own rendering
        Ok(())
    }
    
    fn cleanup(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        println!("Cleaning up Selection Plugin...");
        self.initialized = false;
        
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
