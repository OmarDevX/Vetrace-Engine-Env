use super::*;
#[cfg(feature = "window")]
use super::state::{
    EditorEguiGizmoBridge, EditorEguiGizmoRequest, EditorEguiGizmoShared,
    EditorEguiGizmoTool, EditorEguiGizmoTransform,
};

#[cfg(feature = "window")]
pub(crate) fn install_egui_gizmo_layer(engine: &mut Engine) {
    if !engine.contains_resource::<EguiToolRegistry>() {
        engine.insert_resource(EguiToolRegistry::new());
    }
    if engine.contains_resource::<EditorEguiGizmoBridge>() {
        return;
    }

    let shared = Arc::new(Mutex::new(EditorEguiGizmoShared::default()));
    if let Some(registry) = engine.get_resource::<EguiToolRegistry>().cloned() {
        registry.register(EditorEguiGizmoTool::new(shared.clone()));
    }
    engine.insert_resource(EditorEguiGizmoBridge { shared });
}

#[cfg(not(feature = "window"))]
pub(crate) fn install_egui_gizmo_layer(_engine: &mut Engine) {}

#[cfg(feature = "window")]
pub(crate) fn sync_egui_gizmo_request(engine: &mut Engine) {
    let request = build_egui_gizmo_request(engine);
    if let Some(bridge) = engine.get_resource::<EditorEguiGizmoBridge>() {
        if let Ok(mut shared) = bridge.shared.lock() {
            shared.request = request;
        }
    }
}

#[cfg(not(feature = "window"))]
pub(crate) fn sync_egui_gizmo_request(_engine: &mut Engine) {}

#[cfg(feature = "window")]
fn build_egui_gizmo_request(engine: &Engine) -> Option<EditorEguiGizmoRequest> {
    let state = engine.get_resource::<EditorState>()?;
    if state.selected.is_empty() {
        return None;
    }

    let pivot = selection_center(engine, &state.selected);
    let model_matrix = if state.selected.len() == 1 {
        let entity = state.selected[0];
        global_model_matrix_for(engine, entity).unwrap_or_else(|| Mat4::from_translation(pivot))
    } else {
        Mat4::from_translation(pivot)
    };

    Some(EditorEguiGizmoRequest {
        pivot,
        model_matrix,
        tool: state.active_tool,
        transform_space: state.transform_space,
        interaction_rect_px: engine
            .get_resource::<EditorViewportBounds>()
            .and_then(|bounds| bounds.0),
    })
}

#[cfg(feature = "window")]
pub(crate) fn apply_egui_gizmo_delta(engine: &mut Engine) {
    let pending = engine
        .get_resource::<EditorEguiGizmoBridge>()
        .and_then(|bridge| bridge.shared.lock().ok().and_then(|mut shared| shared.pending_transform.take()));
    let Some(pending) = pending else { return; };

    let state = engine.get_resource::<EditorState>().cloned().unwrap_or_default();
    if state.selected.is_empty() {
        return;
    }

    if state.selected.len() == 1 {
        let entity = state.selected[0];
        set_entity_world_matrix(engine, entity, pending.result_model);
    } else {
        apply_multi_selection_gizmo_transform(engine, &state, pending);
    }

    vetrace_core::propagate_global_transforms(engine);
}

#[cfg(not(feature = "window"))]
pub(crate) fn apply_egui_gizmo_delta(_engine: &mut Engine) {}

#[cfg(feature = "window")]
fn apply_multi_selection_gizmo_transform(engine: &mut Engine, state: &EditorState, pending: EditorEguiGizmoTransform) {
    let delta = pending.result_model * pending.base_model.inverse();
    let (delta_scale, delta_rotation, delta_translation) = safe_decompose_mat4(delta);

    for entity in state.selected.iter().copied() {
        let Some(current_world) = global_model_matrix_for(engine, entity) else { continue; };
        let next_world = match state.multi_pivot {
            EditorMultiPivot::SelectionCenter => delta * current_world,
            EditorMultiPivot::IndividualOrigins => {
                let (scale, rotation, translation) = safe_decompose_mat4(current_world);
                let next = match state.active_tool {
                    EditorTool::Rotate => Mat4::from_scale_rotation_translation(scale, (delta_rotation * rotation).normalize(), translation),
                    EditorTool::Scale => Mat4::from_scale_rotation_translation((scale * delta_scale).max(Vec3::splat(0.01)), rotation, translation),
                    EditorTool::Select | EditorTool::Translate | EditorTool::Omni => Mat4::from_scale_rotation_translation(scale, rotation, translation + delta_translation),
                };
                next
            }
        };
        set_entity_world_matrix(engine, entity, next_world);
    }
}

#[cfg(feature = "window")]
pub(crate) fn egui_gizmo_wants_pointer(engine: &Engine) -> bool {
    engine
        .get_resource::<EditorEguiGizmoBridge>()
        .and_then(|bridge| bridge.shared.lock().ok().map(|shared| shared.wants_pointer || shared.active))
        .unwrap_or(false)
}

#[cfg(not(feature = "window"))]
pub(crate) fn egui_gizmo_wants_pointer(_engine: &Engine) -> bool { false }

#[cfg(feature = "window")]
pub(crate) fn mouse_over_projected_gizmo(engine: &Engine, mouse_px: Vec2) -> bool {
    let Some(request) = build_egui_gizmo_request(engine) else { return false; };
    if request
        .interaction_rect_px
        .is_some_and(|rect| !rect.contains(mouse_px.x, mouse_px.y))
    {
        return false;
    }
    let camera = engine.get_resource::<Camera>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let Some(pivot_px) = project_scene_point_pixels(request.pivot, &camera, &settings) else { return false; };

    // Pre-capture only for the first mouse-down before egui gets to run its exact
    // handle test. Keep it tight so normal object picking still works near the selection.
    if (mouse_px - pivot_px).length() <= 18.0 {
        return true;
    }

    let axis_len = 95.0;
    let axes = gizmo_axes_for_request(request);
    for axis in axes {
        let end_world = request.pivot + axis * 0.75;
        let Some(end_px) = project_scene_point_pixels(end_world, &camera, &settings) else { continue; };
        let dir = normalize_or_2d(end_px - pivot_px, Vec2::X);
        let axis_end = pivot_px + dir * axis_len;
        if point_near_segment(mouse_px, pivot_px, axis_end, 14.0) {
            return true;
        }
    }

    if matches!(request.tool, EditorTool::Rotate | EditorTool::Omni) {
        let radius = 78.0;
        let dist = (mouse_px - pivot_px).length();
        if (dist - radius).abs() <= 14.0 {
            return true;
        }
    }

    false
}

#[cfg(not(feature = "window"))]
pub(crate) fn mouse_over_projected_gizmo(_engine: &Engine, _mouse_px: Vec2) -> bool { false }

#[cfg(feature = "window")]
impl EguiTool for EditorEguiGizmoTool {
    fn layer(&self) -> EguiToolLayer { EguiToolLayer::Viewport }

    fn ui(&mut self, ctx: &egui::Context, frame: &EguiToolContext) {
        let request = self.shared.lock().ok().and_then(|shared| shared.request);
        let Some(request) = request else {
            if let Ok(mut shared) = self.shared.lock() {
                shared.wants_pointer = false;
                if !ctx.input(|i| i.pointer.primary_down()) {
                    shared.active = false;
                }
            }
            return;
        };

        let mode = match request.tool {
            EditorTool::Rotate => egui_gizmo::GizmoMode::Rotate,
            EditorTool::Scale => egui_gizmo::GizmoMode::Scale,
            EditorTool::Omni => egui_gizmo::GizmoMode::Omni,
            EditorTool::Select | EditorTool::Translate => egui_gizmo::GizmoMode::Translate,
        };
        let orientation = match request.transform_space {
            EditorTransformSpace::Local => egui_gizmo::GizmoOrientation::Local,
            EditorTransformSpace::Global => egui_gizmo::GizmoOrientation::Global,
        };

        let full_rect = egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(frame.screen_size_points.x.max(1.0), frame.screen_size_points.y.max(1.0)),
        );
        let viewport = full_rect;
        let interaction_rect = match request.interaction_rect_px {
            // No bounds means a standalone/native editor intentionally uses the
            // complete render window.
            None => full_rect,
            Some(rect) => {
                // An explicit empty viewport means the shell currently has no
                // usable scene area (for example while a docked script panel is
                // being resized). Never fall back to a full-window Area here:
                // doing so would cover every docked egui panel while an entity
                // is selected.
                if rect.is_empty() {
                    clear_inactive_gizmo_pointer_state(&self.shared, ctx);
                    return;
                }
                let points_per_pixel_x = full_rect.width() / frame.surface_size_pixels[0].max(1) as f32;
                let points_per_pixel_y = full_rect.height() / frame.surface_size_pixels[1].max(1) as f32;
                let rect = egui::Rect::from_min_max(
                    egui::pos2(rect.min_x * points_per_pixel_x, rect.min_y * points_per_pixel_y),
                    egui::pos2(rect.max_x * points_per_pixel_x, rect.max_y * points_per_pixel_y),
                )
                .intersect(full_rect);
                if rect.width() <= 0.0 || rect.height() <= 0.0 {
                    clear_inactive_gizmo_pointer_state(&self.shared, ctx);
                    return;
                }
                rect
            }
        };
        let view = Mat4::look_at_rh(frame.camera.position, frame.camera.target, frame.camera.up);
        let aspect = frame.surface_size_pixels[0].max(1) as f32 / frame.surface_size_pixels[1].max(1) as f32;
        let projection = Mat4::perspective_rh(frame.camera.fov_y_radians, aspect, frame.camera.near, frame.camera.far);
        let mut pending = None;

        // Keep the projection viewport in full-window coordinates so the 3D
        // handles line up with the renderer. The egui host Area itself must be
        // limited to the actual scene viewport, though: a full-window
        // Background Area still participates in egui layer hit-testing and can
        // prevent docked Background panels from receiving even hover events.
        //
        // `interaction_rect` comes from the reusable `EditorViewportBounds`
        // resource. Standalone editor users that do not publish bounds keep the
        // previous full-window behavior through the `full_rect` fallback.
        let host_rect = interaction_rect;
        egui::Area::new(egui::Id::new("vetrace_editor_egui_gizmo"))
            .order(egui::Order::Background)
            .fixed_pos(host_rect.min)
            .movable(false)
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_min_size(host_rect.size());
                ui.set_clip_rect(host_rect);
                let gizmo = egui_gizmo::Gizmo::new("vetrace_editor_active_gizmo")
                    .view_matrix(mat4_to_mint(view))
                    .projection_matrix(mat4_to_mint(projection))
                    .model_matrix(mat4_to_mint(request.model_matrix))
                    .viewport(viewport)
                    .interaction_rect(interaction_rect)
                    .orientation(orientation)
                    .mode(mode);

                if let Some(response) = gizmo.interact(ui) {
                    pending = Some(EditorEguiGizmoTransform {
                        base_model: request.model_matrix,
                        result_model: Mat4::from(response.transform()),
                    });
                }
            });

        let pointer_down = ctx.input(|i| i.pointer.primary_down());
        if let Ok(mut shared) = self.shared.lock() {
            let has_pending = pending.is_some();
            shared.active = pointer_down && (shared.active || has_pending);
            if !pointer_down {
                shared.active = false;
            }
            shared.wants_pointer = shared.active || has_pending;
            if let Some(pending) = pending {
                shared.pending_transform = Some(pending);
            }
        }
    }
}

#[cfg(feature = "window")]
fn clear_inactive_gizmo_pointer_state(
    shared: &Arc<Mutex<EditorEguiGizmoShared>>,
    ctx: &egui::Context,
) {
    if let Ok(mut shared) = shared.lock() {
        shared.wants_pointer = false;
        if !ctx.input(|input| input.pointer.primary_down()) {
            shared.active = false;
        }
    }
}

#[cfg(feature = "window")]
pub(crate) fn reset_egui_gizmo_bridge(engine: &mut Engine) {
    if let Some(bridge) = engine.get_resource::<EditorEguiGizmoBridge>() {
        if let Ok(mut shared) = bridge.shared.lock() {
            shared.request = None;
            shared.pending_transform = None;
            shared.wants_pointer = false;
            shared.active = false;
        }
    }
}

#[cfg(not(feature = "window"))]
pub(crate) fn reset_egui_gizmo_bridge(_engine: &mut Engine) {}

#[cfg(feature = "window")]
fn selection_center(engine: &Engine, selected: &[Entity]) -> Vec3 {
    if selected.is_empty() {
        return Vec3::ZERO;
    }
    let mut sum = Vec3::ZERO;
    let mut count = 0.0f32;
    for entity in selected.iter().copied() {
        if let Some(matrix) = global_model_matrix_for(engine, entity) {
            let (_, _, translation) = safe_decompose_mat4(matrix);
            sum += translation;
            count += 1.0;
        }
    }
    if count > 0.0 { sum / count } else { Vec3::ZERO }
}

#[cfg(feature = "window")]
fn global_model_matrix_for(engine: &Engine, entity: Entity) -> Option<Mat4> {
    engine.raw_world().get::<GlobalTransform>(entity)
        .map(global_transform_to_mat4)
        .or_else(|| engine.raw_world().get::<Transform>(entity).map(transform_to_mat4))
}

#[cfg(feature = "window")]
fn set_entity_world_matrix(engine: &mut Engine, entity: Entity, world_matrix: Mat4) {
    let local_matrix = if let Some(parent) = engine.raw_world().get::<Parent>(entity).copied() {
        let parent_world = global_model_matrix_for(engine, parent.0).unwrap_or(Mat4::IDENTITY);
        parent_world.inverse() * world_matrix
    } else {
        world_matrix
    };

    if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
        *transform = transform_from_mat4(local_matrix);
    }
}

#[cfg(feature = "window")]
fn gizmo_axes_for_request(request: EditorEguiGizmoRequest) -> [Vec3; 3] {
    let (_, rotation, _) = safe_decompose_mat4(request.model_matrix);
    if request.transform_space == EditorTransformSpace::Local || request.tool == EditorTool::Scale {
        [
            normalize_or(rotation * Vec3::X, Vec3::X),
            normalize_or(rotation * Vec3::Y, Vec3::Y),
            normalize_or(rotation * Vec3::Z, Vec3::Z),
        ]
    } else {
        [Vec3::X, Vec3::Y, Vec3::Z]
    }
}

#[cfg(feature = "window")]
fn project_scene_point_pixels(point: Vec3, camera: &Camera, settings: &RenderSettings) -> Option<Vec2> {
    let width = settings.width.max(1) as f32;
    let height = settings.height.max(1) as f32;
    let aspect = width / height.max(1.0);
    let view = Mat4::look_at_rh(camera.position, camera.target, camera.up);
    let projection = Mat4::perspective_rh(camera.fov_y_radians, aspect, camera.near, camera.far);
    let clip = projection * view * point.extend(1.0);
    if !clip.w.is_finite() || clip.w <= 0.0 { return None; }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() || ndc.z < -1.0 || ndc.z > 1.0 { return None; }
    Some(Vec2::new(
        (ndc.x * 0.5 + 0.5) * width,
        (1.0 - (ndc.y * 0.5 + 0.5)) * height,
    ))
}

#[cfg(feature = "window")]
fn point_near_segment(point: Vec2, a: Vec2, b: Vec2, radius: f32) -> bool {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 1.0e-6 {
        return (point - a).length() <= radius;
    }
    let t = ((point - a).dot(ab) / len2).clamp(0.0, 1.0);
    let closest = a + ab * t;
    (point - closest).length() <= radius
}

#[cfg(feature = "window")]
fn normalize_or_2d(v: Vec2, fallback: Vec2) -> Vec2 {
    if v.length_squared() > 1.0e-8 { v.normalize() } else { fallback }
}

#[cfg(feature = "window")]
fn normalize_or(v: Vec3, fallback: Vec3) -> Vec3 {
    if v.length_squared() > 1.0e-8 { v.normalize() } else { fallback }
}

#[cfg(feature = "window")]
fn transform_to_mat4(transform: &Transform) -> Mat4 {
    Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, transform.translation)
}

#[cfg(feature = "window")]
fn global_transform_to_mat4(transform: &GlobalTransform) -> Mat4 {
    Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, transform.translation)
}

#[cfg(feature = "window")]
fn transform_from_mat4(matrix: Mat4) -> Transform {
    let (scale, rotation, translation) = safe_decompose_mat4(matrix);
    Transform { translation, rotation, scale: scale.max(Vec3::splat(0.01)) }
}

#[cfg(feature = "window")]
fn safe_decompose_mat4(matrix: Mat4) -> (Vec3, Quat, Vec3) {
    let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
    (
        if scale.is_finite() { scale } else { Vec3::ONE },
        if rotation.is_finite() { rotation.normalize() } else { Quat::IDENTITY },
        if translation.is_finite() { translation } else { Vec3::ZERO },
    )
}

#[cfg(feature = "window")]
fn mat4_to_mint(matrix: Mat4) -> mint::ColumnMatrix4<f32> {
    matrix.into()
}
