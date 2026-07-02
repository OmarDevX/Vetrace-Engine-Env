use crate::components::components::{ObjectRef, Transform};
use crate::ecs::behaviour::Behaviour;
use crate::engine::engine::Engine;
use crate::math::{look_at, perspective};
use glam::{Mat4, Vec3, Quat};
use transform_gizmo_egui::math::Transform as GizmoTransform;
use enumset::EnumSet;
use transform_gizmo_egui::prelude::{enum_set, Gizmo, GizmoConfig, GizmoMode, GizmoOrientation};
use transform_gizmo_egui::config::TransformPivotPoint;
use transform_gizmo_egui::GizmoExt;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorGizmoMode {
    Translate,
    Rotate,
    Scale,
    Omni,
    Arcball,
}

impl EditorGizmoMode {
    pub fn modes(self) -> EnumSet<GizmoMode> {
        match self {
            EditorGizmoMode::Translate => GizmoMode::all_translate(),
            EditorGizmoMode::Rotate => GizmoMode::all_rotate(),
            EditorGizmoMode::Scale => GizmoMode::all_scale(),
            EditorGizmoMode::Omni => GizmoMode::all(),
            EditorGizmoMode::Arcball => enum_set!(GizmoMode::Arcball),
        }
    }
}
pub struct GizmoSystem {
    gizmo: Gizmo,
}

impl GizmoSystem {
    pub fn new() -> Self {
        Self {
            gizmo: Gizmo::default(),
        }
    }

    fn mat4_to_row(m: Mat4) -> mint::RowMatrix4<f64> {
        let c = m.to_cols_array();
        mint::RowMatrix4 {
            x: [c[0] as f64, c[4] as f64, c[8] as f64, c[12] as f64].into(),
            y: [c[1] as f64, c[5] as f64, c[9] as f64, c[13] as f64].into(),
            z: [c[2] as f64, c[6] as f64, c[10] as f64, c[14] as f64].into(),
            w: [c[3] as f64, c[7] as f64, c[11] as f64, c[15] as f64].into(),
        }
    }
}

impl Behaviour for GizmoSystem {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        // Note: Gizmo functionality has been moved to the vetrace_editor crate
        // This is kept for compatibility but doesn't perform actual gizmo operations

        // For now, just return since we don't have selected entities
        return;

        // Note: All gizmo functionality has been moved to vetrace_editor crate
        // The code below is commented out as it references removed main_window field
    }
}