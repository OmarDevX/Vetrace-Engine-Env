use super::*;

/// Current editor manipulation mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTool {
    #[default]
    Select,
    Translate,
    Rotate,
    Scale,
    Omni,
}

/// Compatibility name for older examples that used `EditorGizmoMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorGizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
    Omni,
    Arcball,
}

impl From<EditorGizmoMode> for EditorTool {
    fn from(value: EditorGizmoMode) -> Self {
        match value {
            EditorGizmoMode::Translate => EditorTool::Translate,
            EditorGizmoMode::Rotate => EditorTool::Rotate,
            EditorGizmoMode::Scale => EditorTool::Scale,
            EditorGizmoMode::Omni => EditorTool::Omni,
            EditorGizmoMode::Arcball => EditorTool::Rotate,
        }
    }
}


/// Gizmo orientation space for the active editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorTransformSpace {
    #[default]
    Global,
    Local,
}

/// Multi-selection pivot policy used by the egui gizmo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorMultiPivot {
    #[default]
    SelectionCenter,
    IndividualOrigins,
}


#[cfg(feature = "render_2d")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorViewportMode {
    TwoD,
    #[default]
    ThreeD,
}

/// Editor runtime settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Toggle whether the editor is active.  Inactive mode does not touch input
    /// or selection outlines.
    pub enabled: bool,
    /// If true, the plugin asks the WGPU renderer to unlock/show the cursor via
    /// `RenderSettings`.  Shooter/game runtime code can disable the editor or
    /// override this when it wants a locked FPS cursor.
    pub unlock_cursor: bool,
    /// Outline color used for selected renderable entities.
    pub selection_outline_color: Vec3,
    /// Outline thickness used for selected renderable entities.
    pub selection_outline_thickness: f32,
    /// Base translation speed for keyboard editing.
    pub translate_speed: f32,
    /// Base rotation speed in radians/sec for keyboard editing.
    pub rotate_speed: f32,
    /// Base scale speed per second for keyboard editing.
    pub scale_speed: f32,
    /// If true, selected entities get an editor-managed outline.
    pub draw_selection_outline: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            unlock_cursor: true,
            selection_outline_color: Vec3::new(1.0, 0.85, 0.15),
            selection_outline_thickness: 0.06,
            translate_speed: 3.0,
            rotate_speed: 1.75,
            scale_speed: 1.25,
            draw_selection_outline: true,
        }
    }
}
