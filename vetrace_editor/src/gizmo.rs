//! Gizmo Plugin for Transform Manipulation
//! 
//! This module provides 3D gizmos for manipulating object transforms in the editor.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::components::components::Transform;
use vetrace_engine::math::{look_at, perspective};
use glam::{Mat4, Vec3, Quat};
use transform_gizmo_egui::math::Transform as GizmoTransform;
use enumset::EnumSet;
use transform_gizmo_egui::prelude::{enum_set, Gizmo, GizmoConfig, GizmoMode, GizmoOrientation};
use transform_gizmo_egui::config::TransformPivotPoint;
use transform_gizmo_egui::gizmo::GizmoInteraction;
use egui::{Mesh, PointerButton, Rgba, Sense, Vec2, epaint::Vertex};
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
    
    /// Update and render the gizmo for selected entities.
    ///
    /// `interaction_rect` is the editor scene viewport used to decide whether
    /// the mouse may manipulate the gizmo.  The gizmo projection itself must
    /// still use the full render surface because the engine camera/render path
    /// projects the 3D scene against the full swapchain, not the cropped editor
    /// viewport.  Using the cropped editor rect for both interaction and
    /// projection makes the gizmo appear offset from the selected object.
    pub fn update_gizmo(&mut self, engine: &mut Engine, selected_entities: &[vetrace_engine::ecs::Entity], gizmo_mode: EditorGizmoMode, gizmo_orientation: GizmoOrientation, interaction_rect: egui::Rect) -> bool {
        if selected_entities.is_empty() {
            return false;
        }
        
        // Get camera matrices from the active engine camera
        let view = self.get_view_matrix(engine);
        let proj = self.get_projection_matrix(engine);
        
        // Configure gizmo
        let modes = gizmo_mode.modes();
        let render_viewport_rect = self.get_render_viewport_rect(engine);
        let pivot_point = if gizmo_orientation == GizmoOrientation::Local && selected_entities.len() > 1 {
            TransformPivotPoint::IndividualOrigins
        } else {
            TransformPivotPoint::MedianPoint
        };
        
        self.gizmo.update_config(GizmoConfig {
            view_matrix: Self::mat4_to_row(view),
            projection_matrix: Self::mat4_to_row(proj),
            viewport: render_viewport_rect,
            modes,
            orientation: gizmo_orientation,
            pivot_point,
            ..Default::default()
        });
        
        // Collect transforms
        let mut targets = Vec::new();
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
            }
        }
        
        if targets.is_empty() {
            return false;
        }
        
        // Render and interact with the gizmo in an overlay.
        //
        // Do not use transform_gizmo_egui::GizmoExt::interact() here.  The
        // helper creates a 1x1 egui interaction at the cursor, but it cannot
        // tell the editor selection system whether the cursor is merely over a
        // gizmo handle before the mouse button is pressed.  The editor update
        // phase runs before egui, so selection needs that hover/active state
        // from the previous egui frame to avoid stealing the initial drag click.
        let ctx = &engine.egui_ctx;
        let mut gizmo_captures_pointer = false;
        let mut updated_transforms: Option<Vec<GizmoTransform>> = None;
        
        egui::Area::new("gizmo_overlay".into())
            .fixed_pos(egui::pos2(0.0, 0.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let cursor_pos = ui
                    .ctx()
                    .input(|input| input.pointer.hover_pos())
                    .unwrap_or_default();
                let pointer_in_scene = interaction_rect.contains(cursor_pos);
                let focused_before_update = self.gizmo.is_focused();

                let (drag_started, dragging) = ui.ctx().input(|input| {
                    (
                        pointer_in_scene && input.pointer.button_pressed(PointerButton::Primary),
                        (pointer_in_scene || focused_before_update)
                            && input.pointer.button_down(PointerButton::Primary),
                    )
                });

                if let Some((_res, new_transforms)) = self.gizmo.update(
                    GizmoInteraction {
                        cursor_pos: (cursor_pos.x, cursor_pos.y),
                        hovered: pointer_in_scene,
                        drag_started,
                        dragging,
                    },
                    &targets,
                ) {
                    updated_transforms = Some(new_transforms);
                }

                let over_handle = pointer_in_scene
                    && self.gizmo.pick_preview((cursor_pos.x, cursor_pos.y));

                // Register a tiny egui interaction only while the cursor is on a
                // handle or an active drag is already in progress.  This avoids
                // making the whole scene viewport look like UI input.
                if over_handle || focused_before_update || self.gizmo.is_focused() {
                    let _response = ui.interact(
                        egui::Rect::from_center_size(cursor_pos, Vec2::splat(1.0)),
                        ui.id().with("gizmo_interaction"),
                        Sense::click_and_drag(),
                    );
                }

                gizmo_captures_pointer = over_handle
                    || self.gizmo.is_focused()
                    || updated_transforms.is_some();

                let draw_data = self.gizmo.draw();
                egui::Painter::new(ui.ctx().clone(), ui.layer_id(), render_viewport_rect).add(Mesh {
                    indices: draw_data.indices,
                    vertices: draw_data
                        .vertices
                        .into_iter()
                        .zip(draw_data.colors)
                        .map(|(pos, [r, g, b, a])| Vertex {
                            pos: pos.into(),
                            uv: egui::Pos2::default(),
                            color: Rgba::from_rgba_premultiplied(r, g, b, a).into(),
                        })
                        .collect(),
                    ..Default::default()
                });
            });

        if let Some(new_transforms) = updated_transforms {
            // Apply transforms back to entities
            for (i, new_transform) in new_transforms.iter().enumerate() {
                if i < selected_entities.len() {
                    let entity = selected_entities[i];

                    let desired_global_position = Vec3::new(
                        new_transform.translation.x as f32,
                        new_transform.translation.y as f32,
                        new_transform.translation.z as f32,
                    );
                    let desired_global_rotation = Quat::from_xyzw(
                        new_transform.rotation.v.x as f32,
                        new_transform.rotation.v.y as f32,
                        new_transform.rotation.v.z as f32,
                        new_transform.rotation.s as f32,
                    );

                    // Targets are built from GlobalTransform when available so
                    // the gizmo appears at the rendered world-space location.
                    // If the selected entity has a parent, convert the edited
                    // world-space gizmo result back to the entity's local
                    // Transform; otherwise the hierarchy system will overwrite
                    // the apparent edit on the next frame.
                    let parent_global = engine
                        .world
                        .get::<vetrace_engine::components::components::Parent>(entity)
                        .and_then(|parent| {
                            engine
                                .world
                                .get::<vetrace_engine::components::components::GlobalTransform>(parent.entity)
                                .copied()
                        });

                    let (local_position, local_rotation) = if let Some(parent_global) = parent_global {
                        let parent_position = Vec3::from(parent_global.position);
                        let parent_rotation = Quat::from_xyzw(
                            parent_global.orientation[0],
                            parent_global.orientation[1],
                            parent_global.orientation[2],
                            parent_global.orientation[3],
                        );
                        let inv_parent_rotation = parent_rotation.conjugate();
                        (
                            inv_parent_rotation * (desired_global_position - parent_position),
                            (inv_parent_rotation * desired_global_rotation).normalize(),
                        )
                    } else {
                        (desired_global_position, desired_global_rotation.normalize())
                    };

                    if let Some(mut transform) = engine.world.get_mut::<Transform>(entity) {
                        transform.position = local_position.to_array();
                        transform.orientation = [
                            local_rotation.x,
                            local_rotation.y,
                            local_rotation.z,
                            local_rotation.w,
                        ];
                        transform.size = [
                            new_transform.scale.x as f32,
                            new_transform.scale.y as f32,
                            new_transform.scale.z as f32,
                        ];

                        // Mark BVH dirty so raytracing bounds rebuild after gizmo edits
                        engine.scene.bvh_dirty = true;
                    }
                }
            }
        }
        
        gizmo_captures_pointer
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
    
    /// Get the viewport used by the 3D render surface.
    fn get_render_viewport_rect(&self, engine: &Engine) -> egui::Rect {
        let (width, height) = engine.window.window.size();
        egui::Rect::from_min_max(
            egui::Pos2::ZERO,
            egui::pos2(width as f32, height as f32),
        )
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