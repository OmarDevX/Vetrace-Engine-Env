use std::any::Any;
use std::collections::HashMap;
use std::error::Error;
#[cfg(feature = "window")]
use std::sync::{Arc, Mutex};

use glam::{Mat4, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use vetrace_core::app::Plugin;
use vetrace_core::components::builtins::{GlobalTransform, Name, Parent, Transform};
use vetrace_core::engine::{ComponentManager, Engine};
use vetrace_core::{Entity, InputState};
use vetrace_render::{
    Camera, EguiOverlayPanel, Material, Outline, PrimitiveShape, ReflectionProbe,
    ReflectionProbeCaptureRequests, RenderSettings, RenderStats, Renderable, ScreenSpaceRect, Shape,
};
#[cfg(feature = "window")]
use vetrace_render::{egui, EguiTool, EguiToolContext, EguiToolLayer, EguiToolRegistry};
#[cfg(feature = "render_2d")]
use vetrace_render::{Camera2D, CanvasItem2D, Sprite2D};
#[cfg(feature = "physics_2d")]
use vetrace_physics::{Collider2D, ColliderShape2D};

mod config;
mod gizmo_egui;
mod helpers;
mod overlay;
mod picking;
#[cfg(feature = "render_2d")]
mod picking_2d;
#[cfg(feature = "render_2d")]
mod selection_2d;
mod plugin;
mod selection;
mod state;
mod transform_tools;

use gizmo_egui::{apply_egui_gizmo_delta, egui_gizmo_wants_pointer, install_egui_gizmo_layer, mouse_over_projected_gizmo, reset_egui_gizmo_bridge, sync_egui_gizmo_request};
use overlay::{refresh_egui_overlay, refresh_status};
use picking::{entity_label, global_transform_for, pick_entity_from_mouse, primitive_radius_for, sphere_intersection};
use selection::{apply_editor_outlines, cycle_selection, delete_selected, restore_editor_outlines, selectable_entities, set_selected, set_tool};
#[cfg(feature = "render_2d")]
use selection_2d::{hide_2d_selection_overlay, install_2d_selection_overlay, refresh_2d_selection_overlay};
use state::EditorOutlineBackups;
use transform_tools::apply_keyboard_transform;
#[cfg(feature = "render_2d")]
use transform_tools::{apply_keyboard_transform_2d, apply_pointer_transform_2d};

pub use config::{EditorConfig, EditorGizmoMode, EditorMultiPivot, EditorTool, EditorTransformSpace};
#[cfg(feature = "render_2d")]
pub use config::EditorViewportMode;
pub use helpers::{editor_view_matrix, spawn_editor_overlay_marker, spawn_editor_test_cube};
pub use plugin::{editor, EditorCameraBookmark, EditorOnly, EditorPlugin};
pub use state::{EditorKeyboardCapture, EditorPointerCapture, EditorState, EditorViewportBounds, EditorViewportRect};


/// Select an entity through the editor's normal outline and state path.
pub fn select_editor_entity(engine: &mut Engine, entity: Option<Entity>) {
    let config = engine
        .get_resource::<EditorConfig>()
        .cloned()
        .unwrap_or_default();
    set_selected(engine, entity, &config);
}

/// Remove temporary editor selection visuals before scene serialization.
pub fn prepare_editor_scene_export(engine: &mut Engine) {
    restore_editor_outlines(engine);
}

/// Restore selection visuals after scene serialization.
pub fn restore_editor_selection_visuals(engine: &mut Engine) {
    let config = engine
        .get_resource::<EditorConfig>()
        .cloned()
        .unwrap_or_default();
    apply_editor_outlines(engine, &config);
}

/// Requests a runtime/baked recapture for the currently selected reflection
/// probe. Returns false when no selected entity owns a probe.
pub fn request_selected_reflection_probe_capture(engine: &mut Engine) -> bool {
    let Some(entity) = engine
        .get_resource::<EditorState>()
        .and_then(|state| state.selected_primary())
    else {
        return false;
    };
    if !engine.raw_world().has::<ReflectionProbe>(entity) {
        return false;
    }
    if let Some(requests) = engine.get_resource_mut::<ReflectionProbeCaptureRequests>() {
        requests.request(entity);
        true
    } else {
        false
    }
}
