use super::*;

/// Editor state stored as an engine resource.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EditorState {
    pub selected: Vec<Entity>,
    pub hovered: Option<Entity>,
    pub active_tool: EditorTool,
    #[cfg(feature = "render_2d")]
    pub viewport_mode: EditorViewportMode,
    pub transform_space: EditorTransformSpace,
    pub multi_pivot: EditorMultiPivot,
    pub last_pick_distance: Option<f32>,
    pub status: String,
}

impl EditorState {
    pub fn selected_primary(&self) -> Option<Entity> { self.selected.first().copied() }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.last_pick_distance = None;
    }

    pub fn set_selected(&mut self, entity: Option<Entity>) {
        self.selected.clear();
        if let Some(entity) = entity {
            self.selected.push(entity);
        }
    }
}


/// Set by higher-level editor shells when an egui widget, popup, menu, or
/// modal owns the pointer. Static docked regions should be described through
/// `EditorViewportBounds` instead of being folded into this flag.
#[derive(Clone, Copy, Debug, Default)]
pub struct EditorPointerCapture(pub bool);

/// Set by higher-level editor shells while an egui text field or keyboard
/// widget is focused. Editor shortcuts and destructive keys must not leak
/// through into scene editing while this is true.
#[derive(Clone, Copy, Debug, Default)]
pub struct EditorKeyboardCapture(pub bool);

/// Physical-pixel bounds of the interactive scene viewport supplied by a
/// higher-level editor shell. Rendering may still cover the whole window, but
/// picking, camera controls, and gizmo hit-testing must stay inside this rect.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorViewportRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl EditorViewportRect {
    pub fn width(self) -> f32 { self.max_x - self.min_x }

    pub fn height(self) -> f32 { self.max_y - self.min_y }

    /// Returns true when the shell has explicitly published a viewport with no
    /// usable area. This is different from `EditorViewportBounds(None)`, which
    /// means an unbounded/full-window native editor integration.
    pub fn is_empty(self) -> bool {
        !self.min_x.is_finite()
            || !self.min_y.is_finite()
            || !self.max_x.is_finite()
            || !self.max_y.is_finite()
            || self.width() <= 0.0
            || self.height() <= 0.0
    }

    pub fn contains(self, x: f32, y: f32) -> bool {
        !self.is_empty()
            && x >= self.min_x
            && x < self.max_x
            && y >= self.min_y
            && y < self.max_y
    }
}

/// Optional viewport bounds. `None` preserves the full-window behavior used by
/// standalone/native editor integrations that do not have docked UI panels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EditorViewportBounds(pub Option<EditorViewportRect>);

impl EditorViewportBounds {
    /// Returns `true` when a physical-pixel pointer position belongs to the
    /// interactive scene viewport. Unbounded editor integrations accept the
    /// complete window.
    pub fn contains(self, x: f32, y: f32) -> bool {
        match self.0 {
            Some(rect) => rect.contains(x, y),
            None => true,
        }
    }

    pub fn blocks_pointer(self, x: f32, y: f32) -> bool { !self.contains(x, y) }
}

#[cfg(test)]
mod tests {
    use super::{EditorViewportBounds, EditorViewportRect};

    #[test]
    fn viewport_rect_uses_half_open_bounds() {
        let bounds = EditorViewportBounds(Some(EditorViewportRect {
            min_x: 100.0,
            min_y: 50.0,
            max_x: 900.0,
            max_y: 650.0,
        }));

        assert!(bounds.contains(100.0, 50.0));
        assert!(bounds.contains(899.9, 649.9));
        assert!(!bounds.contains(99.9, 50.0));
        assert!(!bounds.contains(900.0, 300.0));
        assert!(!bounds.contains(300.0, 650.0));
    }

    #[test]
    fn explicit_empty_viewport_blocks_all_pointer_input() {
        let bounds = EditorViewportBounds(Some(EditorViewportRect {
            min_x: 300.0,
            min_y: 400.0,
            max_x: 300.0,
            max_y: 250.0,
        }));

        assert!(bounds.0.unwrap().is_empty());
        assert!(!bounds.contains(300.0, 300.0));
        assert!(bounds.blocks_pointer(300.0, 300.0));
    }

    #[test]
    fn missing_viewport_bounds_preserve_full_window_input() {
        let bounds = EditorViewportBounds::default();
        assert!(bounds.contains(-100.0, -100.0));
        assert!(!bounds.blocks_pointer(10_000.0, 10_000.0));
    }
}

#[derive(Default)]
pub(crate) struct EditorOutlineBackups {
    pub(crate) previous: HashMap<Entity, Option<Outline>>,
}

#[cfg(feature = "window")]
#[derive(Clone)]
pub(crate) struct EditorEguiGizmoBridge {
    pub(crate) shared: Arc<Mutex<EditorEguiGizmoShared>>,
}

#[cfg(feature = "window")]
#[derive(Clone, Debug, Default)]
pub(crate) struct EditorEguiGizmoShared {
    pub(crate) request: Option<EditorEguiGizmoRequest>,
    pub(crate) pending_transform: Option<EditorEguiGizmoTransform>,
    pub(crate) wants_pointer: bool,
    pub(crate) active: bool,
}

#[cfg(feature = "window")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorEguiGizmoRequest {
    pub(crate) pivot: Vec3,
    pub(crate) model_matrix: Mat4,
    pub(crate) tool: EditorTool,
    pub(crate) transform_space: EditorTransformSpace,
    pub(crate) interaction_rect_px: Option<EditorViewportRect>,
}

#[cfg(feature = "window")]
#[derive(Clone, Copy, Debug)]
pub(crate) struct EditorEguiGizmoTransform {
    pub(crate) base_model: Mat4,
    pub(crate) result_model: Mat4,
}

#[cfg(feature = "window")]
pub(crate) struct EditorEguiGizmoTool {
    pub(crate) shared: Arc<Mutex<EditorEguiGizmoShared>>,
}

#[cfg(feature = "window")]
impl EditorEguiGizmoTool {
    pub(crate) fn new(shared: Arc<Mutex<EditorEguiGizmoShared>>) -> Self {
        Self { shared }
    }
}
