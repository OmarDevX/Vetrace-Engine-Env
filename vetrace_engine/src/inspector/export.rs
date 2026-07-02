use std::any::TypeId;

/// UI export kind (slider, checkbox, etc)
#[derive(Debug)]
pub enum ExportKind {
    Slider {
        min: f32,
        max: f32,
    },
    /// Numeric drag editor for large ranges that need precise manual authoring.
    Drag {
        min: f32,
        max: f32,
        speed: f32,
    },
    Checkbox,
    Text,
    Dropdown(Vec<String>),
    // Add more types as needed
}

/// This struct represents a field to be shown in UI
pub struct ExportedField {
    pub name: &'static str,
    pub kind: ExportKind,
    pub value: *mut dyn std::any::Any,
    pub type_id: TypeId,
}
