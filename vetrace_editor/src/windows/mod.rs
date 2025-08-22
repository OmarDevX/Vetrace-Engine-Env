//! Editor Windows Module
//! 
//! This module contains all the editor window implementations,
//! moved from the main engine to keep editor functionality separate.

use vetrace_engine::{ecs::Entity, engine::engine::Engine, scene::object::Object};
use egui::{
    ComboBox, Context, Margin, ScrollArea, Slider, TextEdit, TopBottomPanel, Ui, Window,
};
use rfd::FileDialog;
use std::collections::HashMap;
use transform_gizmo_egui::GizmoOrientation;

pub mod main_window;
pub mod sandbox_window;
// pub mod file_explorer;
// pub mod component_editor;

pub use main_window::MainWindow;
pub use sandbox_window::SandboxWindow;

use crate::EditorWindow;

/// Represents a new field being created in the component editor
#[derive(Clone)]
pub struct NewField {
    pub name: String,
    pub ty_index: usize,
    pub default: String,
}

impl NewField {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            ty_index: 0,
            default: String::new(),
        }
    }
}

impl Default for NewField {
    fn default() -> Self {
        Self::new()
    }
}

/// Editor gizmo modes
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorGizmoMode {
    Translate,
    Rotate,
    Scale,
    Omni,
    Arcball,
}

impl EditorGizmoMode {
    pub fn modes(&self) -> enumset::EnumSet<transform_gizmo_egui::GizmoMode> {
        use transform_gizmo_egui::prelude::{enum_set, GizmoMode};
        match self {
            EditorGizmoMode::Translate => enum_set!(GizmoMode::TranslateX | GizmoMode::TranslateY | GizmoMode::TranslateZ),
            EditorGizmoMode::Rotate => enum_set!(GizmoMode::RotateX | GizmoMode::RotateY | GizmoMode::RotateZ),
            EditorGizmoMode::Scale => enum_set!(GizmoMode::ScaleX | GizmoMode::ScaleY | GizmoMode::ScaleZ),
            EditorGizmoMode::Omni => enum_set!(GizmoMode::TranslateX | GizmoMode::TranslateY | GizmoMode::TranslateZ | GizmoMode::RotateX | GizmoMode::RotateY | GizmoMode::RotateZ | GizmoMode::ScaleX | GizmoMode::ScaleY | GizmoMode::ScaleZ),
            EditorGizmoMode::Arcball => enum_set!(GizmoMode::RotateX | GizmoMode::RotateY | GizmoMode::RotateZ),
        }
    }
}

impl Default for EditorGizmoMode {
    fn default() -> Self {
        EditorGizmoMode::Translate
    }
}

/// Common UI utilities for editor windows
pub struct EditorUI;

impl EditorUI {
    /// Draw a labeled text input field
    pub fn labeled_text_input(ui: &mut Ui, label: &str, text: &mut String) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.text_edit_singleline(text).changed()
        }).inner
    }
    
    /// Draw a labeled slider
    pub fn labeled_slider<T>(ui: &mut Ui, label: &str, value: &mut T, range: std::ops::RangeInclusive<T>) -> bool 
    where 
        T: egui::emath::Numeric 
    {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(Slider::new(value, range)).changed()
        }).inner
    }
    
    /// Draw a labeled checkbox
    pub fn labeled_checkbox(ui: &mut Ui, label: &str, checked: &mut bool) -> bool {
        ui.checkbox(checked, label).changed()
    }
    
    /// Draw a labeled combo box
    pub fn labeled_combo_box<T>(
        ui: &mut Ui, 
        label: &str, 
        selected: &mut T, 
        options: &[T],
        display_fn: impl Fn(&T) -> String
    ) -> bool 
    where 
        T: PartialEq + Clone 
    {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(label);
            ComboBox::from_id_source(label)
                .selected_text(display_fn(selected))
                .show_ui(ui, |ui| {
                    for option in options {
                        if ui.selectable_value(selected, option.clone(), display_fn(option)).changed() {
                            changed = true;
                        }
                    }
                });
        });
        changed
    }
    
    /// Draw a collapsible section
    pub fn collapsible_section<R>(
        ui: &mut Ui,
        title: &str,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        ui.collapsing(title, add_contents).body_returned
    }
    
    /// Draw a separator with optional label
    pub fn separator_with_label(ui: &mut Ui, label: Option<&str>) {
        if let Some(label) = label {
            ui.separator();
            ui.label(label);
            ui.separator();
        } else {
            ui.separator();
        }
    }
    
    /// Draw a button with icon (if available)
    pub fn icon_button(ui: &mut Ui, text: &str, icon: Option<&str>) -> bool {
        let button_text = if let Some(icon) = icon {
            format!("{} {}", icon, text)
        } else {
            text.to_string()
        };
        ui.button(button_text).clicked()
    }
    
    /// Draw a tooltip on hover
    pub fn tooltip(ui: &mut Ui, text: &str) {
        // ui.on_hover_text(text); // TODO: Fix hover text API
    }
    
    /// Draw a help marker with tooltip
    pub fn help_marker(ui: &mut Ui, help_text: &str) {
        ui.label("(?)").on_hover_text(help_text);
    }
    
    /// Draw a color picker
    pub fn color_picker(ui: &mut Ui, label: &str, color: &mut [f32; 3]) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            let mut egui_color = egui::Color32::from_rgb(
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
            );
            let changed = ui.color_edit_button_srgba(&mut egui_color).changed();
            if changed {
                let [r, g, b, _] = egui_color.to_array();
                color[0] = r as f32 / 255.0;
                color[1] = g as f32 / 255.0;
                color[2] = b as f32 / 255.0;
            }
            changed
        }).inner
    }
    
    /// Draw a vector3 input
    pub fn vec3_input(ui: &mut Ui, label: &str, vec: &mut [f32; 3]) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            let x_changed = ui.add(egui::DragValue::new(&mut vec[0]).speed(0.1)).changed();
            let y_changed = ui.add(egui::DragValue::new(&mut vec[1]).speed(0.1)).changed();
            let z_changed = ui.add(egui::DragValue::new(&mut vec[2]).speed(0.1)).changed();
            x_changed || y_changed || z_changed
        }).inner
    }
    
    /// Draw a file path selector
    pub fn file_path_selector(ui: &mut Ui, label: &str, path: &mut String, filter: Option<&str>) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            let text_changed = ui.text_edit_singleline(path).changed();
            let button_clicked = ui.button("Browse").clicked();
            
            if button_clicked {
                let mut dialog = FileDialog::new();
                if let Some(filter) = filter {
                    dialog = dialog.add_filter("Files", &[filter]);
                }
                if let Some(file_path) = dialog.pick_file() {
                    *path = file_path.to_string_lossy().to_string();
                    return true;
                }
            }
            
            text_changed
        }).inner
    }
}
