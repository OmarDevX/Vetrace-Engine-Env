//! UI Components for the Editor
//! 
//! This module provides reusable UI components for the editor interface.

use egui::{Ui, Response};

/// A collapsible header with custom styling
pub struct CollapsibleHeader {
    title: String,
    open: bool,
}

impl CollapsibleHeader {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            open: false,
        }
    }
    
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
    
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        ui.collapsing(&self.title, add_contents).body_returned
    }
}

/// A property editor for different value types
pub struct PropertyEditor;

impl PropertyEditor {
    /// Edit a float value with a slider
    pub fn float_slider(
        ui: &mut Ui,
        label: &str,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
    ) -> Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::Slider::new(value, range))
        }).inner
    }
    
    /// Edit a float value with a drag widget
    pub fn float_drag(
        ui: &mut Ui,
        label: &str,
        value: &mut f32,
        speed: f32,
    ) -> Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).speed(speed))
        }).inner
    }
    
    /// Edit a Vec3 value
    pub fn vec3(
        ui: &mut Ui,
        label: &str,
        value: &mut [f32; 3],
        speed: f32,
    ) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            let x = ui.add(egui::DragValue::new(&mut value[0]).speed(speed)).changed();
            let y = ui.add(egui::DragValue::new(&mut value[1]).speed(speed)).changed();
            let z = ui.add(egui::DragValue::new(&mut value[2]).speed(speed)).changed();
            x || y || z
        }).inner
    }
    
    /// Edit a color value
    pub fn color(
        ui: &mut Ui,
        label: &str,
        color: &mut [f32; 3],
    ) -> bool {
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
    
    /// Edit a boolean value
    pub fn boolean(
        ui: &mut Ui,
        label: &str,
        value: &mut bool,
    ) -> Response {
        ui.checkbox(value, label)
    }
    
    /// Edit a string value
    pub fn string(
        ui: &mut Ui,
        label: &str,
        value: &mut String,
    ) -> Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.text_edit_singleline(value)
        }).inner
    }
    
    /// Edit an integer value
    pub fn integer(
        ui: &mut Ui,
        label: &str,
        value: &mut i32,
        speed: f32,
    ) -> Response {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::DragValue::new(value).speed(speed))
        }).inner
    }
}

/// A file picker widget
pub struct FilePicker {
    label: String,
    filter: Option<String>,
    current_path: String,
}

impl FilePicker {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            filter: None,
            current_path: String::new(),
        }
    }
    
    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }
    
    pub fn current_path(mut self, path: impl Into<String>) -> Self {
        self.current_path = path.into();
        self
    }
    
    pub fn show(mut self, ui: &mut Ui) -> Option<String> {
        ui.horizontal(|ui| {
            ui.label(&self.label);
            ui.text_edit_singleline(&mut self.current_path);
            
            if ui.button("Browse").clicked() {
                let mut dialog = rfd::FileDialog::new();
                if let Some(filter) = &self.filter {
                    dialog = dialog.add_filter("Files", &[filter]);
                }
                if let Some(path) = dialog.pick_file() {
                    self.current_path = path.to_string_lossy().to_string();
                    return Some(self.current_path.clone());
                }
            }
            None
        }).inner
    }
}

/// A toolbar widget
pub struct Toolbar {
    items: Vec<ToolbarItem>,
}

#[derive(Clone)]
pub struct ToolbarItem {
    pub label: String,
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    pub enabled: bool,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
        }
    }
    
    pub fn add_item(mut self, item: ToolbarItem) -> Self {
        self.items.push(item);
        self
    }
    
    pub fn add_button(mut self, label: impl Into<String>) -> Self {
        self.items.push(ToolbarItem {
            label: label.into(),
            icon: None,
            tooltip: None,
            enabled: true,
        });
        self
    }
    
    pub fn show(self, ui: &mut Ui) -> Vec<bool> {
        let mut clicked = vec![false; self.items.len()];
        
        ui.horizontal(|ui| {
            for (i, item) in self.items.iter().enumerate() {
                ui.add_enabled_ui(item.enabled, |ui| {
                    let button_text = if let Some(icon) = &item.icon {
                        format!("{} {}", icon, item.label)
                    } else {
                        item.label.clone()
                    };
                    
                    let response = ui.button(button_text);

                    let was_clicked = response.clicked();

                    if let Some(tooltip) = &item.tooltip {
                        response.on_hover_text(tooltip);
                    }

                    if was_clicked {
                        clicked[i] = true;
                    }
                });
                
                if i < self.items.len() - 1 {
                    ui.separator();
                }
            }
        });
        
        clicked
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

/// A status bar widget
pub struct StatusBar {
    left_items: Vec<String>,
    right_items: Vec<String>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left_items: Vec::new(),
            right_items: Vec::new(),
        }
    }
    
    pub fn add_left(mut self, text: impl Into<String>) -> Self {
        self.left_items.push(text.into());
        self
    }
    
    pub fn add_right(mut self, text: impl Into<String>) -> Self {
        self.right_items.push(text.into());
        self
    }
    
    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Left items
            for item in &self.left_items {
                ui.label(item);
                ui.separator();
            }
            
            // Spacer
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Right items (in reverse order due to right-to-left layout)
                for item in self.right_items.iter().rev() {
                    ui.label(item);
                    ui.separator();
                }
            });
        });
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
