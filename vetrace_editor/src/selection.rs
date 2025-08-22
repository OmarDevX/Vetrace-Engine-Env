//! Selection Plugin for Entity Selection
//! 
//! This module provides entity selection functionality through mouse picking.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::ecs::Entity;
use sdl2::mouse::MouseButton;
use vetrace_engine::math::{array_to_vec3, vec3_to_array};
use glam::{Vec3, Quat};

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
    
    /// Create a ray from screen coordinates
    fn screen_ray(&self, engine: &Engine, x: f32, y: f32) -> (Vec3, Vec3) {
        // This would need to be implemented to convert screen coordinates to world ray
        // For now, return a default ray
        let origin = Vec3::new(0.0, 0.0, 5.0);
        let direction = Vec3::new(0.0, 0.0, -1.0);
        (origin, direction)
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
    
    /// Test ray-cube intersection
    fn cube_intersect(&self, origin: Vec3, dir: Vec3, center: Vec3, size: Vec3, _rotation: Quat) -> Option<f32> {
        // Simplified AABB intersection for now
        let min = center - size * 0.5;
        let max = center + size * 0.5;
        
        let inv_dir = Vec3::new(1.0 / dir.x, 1.0 / dir.y, 1.0 / dir.z);
        
        let t1 = (min - origin) * inv_dir;
        let t2 = (max - origin) * inv_dir;
        
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
    
    /// Test ray-mesh intersection
    fn mesh_intersect(&self, engine: &Engine, origin: Vec3, dir: Vec3, pos: Vec3, orient: Quat, obj: &vetrace_engine::scene::object::Object) -> Option<f32> {
        // This would need to be implemented for mesh intersection
        // For now, fall back to sphere intersection
        let radius = obj.size[0].max(obj.size[1]).max(obj.size[2]) * 0.5;
        self.sphere_intersect(origin, dir, pos, radius)
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
