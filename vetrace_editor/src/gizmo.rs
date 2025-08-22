//! Gizmo Plugin for Transform Manipulation
//! 
//! This module provides 3D gizmos for manipulating object transforms in the editor.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::components::components::{ObjectRef, Transform};
use vetrace_engine::math::{look_at, perspective};
use glam::{Mat4, Vec3, Quat};
use transform_gizmo_egui::math::Transform as GizmoTransform;
use enumset::EnumSet;
use transform_gizmo_egui::prelude::{enum_set, Gizmo, GizmoConfig, GizmoMode, GizmoOrientation};
use transform_gizmo_egui::config::TransformPivotPoint;
use transform_gizmo_egui::GizmoExt;
use mint::{Vector3, Quaternion};

use vetrace_engine::systems::gizmo::EditorGizmoMode;

/// Gizmo plugin for transform manipulation
pub struct GizmoPlugin {
    gizmo: Gizmo,
    initialized: bool,
}

impl GizmoPlugin {
    /// Create a new gizmo plugin
    pub fn new() -> Self {
        Self {
            gizmo: Gizmo::default(),
            initialized: false,
        }
    }
    
    /// Update and render the gizmo for selected entities
    pub fn update_gizmo(&mut self, engine: &mut Engine, selected_entities: &[vetrace_engine::ecs::Entity], gizmo_mode: EditorGizmoMode, gizmo_orientation: GizmoOrientation) -> bool {
        if selected_entities.is_empty() {
            return false;
        }
        
        // Get camera matrices from the active engine camera
        let view = self.get_view_matrix(engine);
        let proj = self.get_projection_matrix(engine);
        let viewport = self.get_viewport(engine);
        
        // Configure gizmo
        let modes = gizmo_mode.modes();
        let pivot_point = if gizmo_orientation == GizmoOrientation::Local && selected_entities.len() > 1 {
            TransformPivotPoint::IndividualOrigins
        } else {
            TransformPivotPoint::MedianPoint
        };
        
        self.gizmo.update_config(GizmoConfig {
            view_matrix: Self::mat4_to_row(view),
            projection_matrix: Self::mat4_to_row(proj),
            viewport: egui::Rect::from_min_size(
                egui::Pos2::new(viewport[0], viewport[1]),
                egui::Vec2::new(viewport[2], viewport[3])
            ),
            modes,
            orientation: gizmo_orientation,
            pivot_point,
            ..Default::default()
        });
        
        // Collect transforms
        let mut targets = Vec::new();
        let mut originals = Vec::new();
        
        for &entity in selected_entities {
            if let Some(transform) = engine.world.get::<Transform>(entity) {
                // Prefer global transform if available so gizmos match world space
                let (position, orientation, scale) = if let Some(global) = engine
                    .world
                    .get::<vetrace_engine::components::components::GlobalTransform>(entity)
                {
                    (global.position, global.orientation, global.size)
                } else {
                    (transform.position, transform.orientation, transform.size)
                };

                let gizmo_transform = GizmoTransform {
                    translation: Vector3 {
                        x: position[0] as f64,
                        y: position[1] as f64,
                        z: position[2] as f64,
                    },
                    rotation: Quaternion {
                        s: orientation[3] as f64, // w component
                        v: Vector3 {
                            x: orientation[0] as f64,
                            y: orientation[1] as f64,
                            z: orientation[2] as f64,
                        },
                    },
                    scale: Vector3 {
                        x: scale[0] as f64,
                        y: scale[1] as f64,
                        z: scale[2] as f64,
                    },
                };
                targets.push(gizmo_transform);
                originals.push(gizmo_transform);
            }
        }
        
        if targets.is_empty() {
            return false;
        }
        
        // Render gizmo in an overlay
        let ctx = &engine.egui_ctx;
        let mut gizmo_hovered = false;
        
        egui::Area::new("gizmo_overlay".into())
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                if let Some((_res, new_transforms)) = self.gizmo.interact(ui, &targets) {
                    // Apply transforms back to entities
                    for (i, new_transform) in new_transforms.iter().enumerate() {
                        if i < selected_entities.len() {
                            let entity = selected_entities[i];
                            
                            // Get object reference id before mutable borrow
                            let obj_ref_id = engine.world.get::<ObjectRef>(entity).map(|obj_ref| obj_ref.id);

                            if let Some(mut transform) = engine.world.get_mut::<Transform>(entity) {
                                transform.position = [
                                    new_transform.translation.x as f32,
                                    new_transform.translation.y as f32,
                                    new_transform.translation.z as f32,
                                ];

                                let rot = Quat::from_xyzw(
                                    new_transform.rotation.v.x as f32,
                                    new_transform.rotation.v.y as f32,
                                    new_transform.rotation.v.z as f32,
                                    new_transform.rotation.s as f32,
                                );

                                transform.orientation = [rot.x, rot.y, rot.z, rot.w];
                                transform.size = [
                                    new_transform.scale.x as f32,
                                    new_transform.scale.y as f32,
                                    new_transform.scale.z as f32,
                                ];

                                // Update object position if it has an ObjectRef
                                if let Some(obj_id) = obj_ref_id {
                                    if let Some(obj) = engine.scene.objects.get_mut(obj_id as usize) {
                                        obj.position = transform.position;
                                        obj.orientation = transform.orientation;
                                        obj.scale = transform.size;
                                    }
                                }
                            }
                        }
                    }
                }
                
                gizmo_hovered = ui.rect_contains_pointer(ui.max_rect());
            });
        
        gizmo_hovered
    }
    
    /// Get the view matrix from the engine's active camera
    fn get_view_matrix(&self, engine: &Engine) -> Mat4 {
        let cam = engine.active_camera_info();
        let eye = cam.position;
        let front = cam.orientation * Vec3::X;
        let up = cam.orientation * Vec3::Y;
        look_at(&eye, &(eye + front), &up)
    }

    /// Get the projection matrix from the engine's active camera
    fn get_projection_matrix(&self, engine: &Engine) -> Mat4 {
        let cam = engine.active_camera_info();
        let (width, height) = engine.window.window.size();
        perspective(cam.fov, width as f32 / height as f32, 0.1, 1000.0)
    }
    
    /// Get the viewport from the engine
    fn get_viewport(&self, engine: &Engine) -> [f32; 4] {
        let (width, height) = engine.window.window.size();
        [0.0, 0.0, width as f32, height as f32]
    }
    
    /// Convert Mat4 to row-major matrix for gizmo
    fn mat4_to_row(m: Mat4) -> mint::RowMatrix4<f64> {
        let arr = m.to_cols_array();
        mint::RowMatrix4 {
            x: mint::Vector4 { x: arr[0] as f64, y: arr[4] as f64, z: arr[8] as f64, w: arr[12] as f64 },
            y: mint::Vector4 { x: arr[1] as f64, y: arr[5] as f64, z: arr[9] as f64, w: arr[13] as f64 },
            z: mint::Vector4 { x: arr[2] as f64, y: arr[6] as f64, z: arr[10] as f64, w: arr[14] as f64 },
            w: mint::Vector4 { x: arr[3] as f64, y: arr[7] as f64, z: arr[11] as f64, w: arr[15] as f64 },
        }
    }
}

impl Default for GizmoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for GizmoPlugin {
    fn name(&self) -> &'static str {
        "gizmo"
    }
    
    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if self.initialized {
            return Ok(());
        }
        
        println!("Initializing Gizmo Plugin...");
        
        // Initialize gizmo
        self.gizmo = Gizmo::default();
        self.initialized = true;
        
        Ok(())
    }
    
    fn update(&mut self, _engine: &mut Engine, _delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Gizmo update is handled by the main window
        Ok(())
    }
    
    fn render(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Gizmo rendering is handled by the update_gizmo method
        Ok(())
    }
    
    fn cleanup(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        println!("Cleaning up Gizmo Plugin...");
        self.initialized = false;
        
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
