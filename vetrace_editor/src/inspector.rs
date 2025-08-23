//! Inspector Plugin for Component Editing
//! 
//! This module provides the inspector functionality for editing component properties.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::ecs::Entity;

/// Inspector plugin for component editing
pub struct InspectorPlugin {
    initialized: bool,
}

impl InspectorPlugin {
    /// Create a new inspector plugin
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }
    
    /// Draw component UI for a specific entity
    pub fn draw_component_ui(&self, ui: &mut egui::Ui, engine: &mut Engine, entity: Entity) {
        // Get all components for this entity
        let component_names = engine.list_components_entity(entity);
        
        for name in component_names {
            if let Some(editor_fn) = engine.component_editors.get(&name).cloned() {
                ui.collapsing(&name, |ui| {
                    editor_fn(engine, entity, ui);
                });
            }
        }
    }
    
    /// Draw the add component UI
    pub fn draw_add_component_ui(&self, ui: &mut egui::Ui, engine: &mut Engine, entity: Entity, selected_component: &mut String) {
        ui.label("Add Component:");
        
        // Get available components
        let mut names: Vec<_> = engine.component_adders.keys().cloned().collect();
        for g in &engine.generated_components {
            if !names.contains(g) {
                names.push(g.clone());
            }
        }
        
        // Component selector
        egui::ComboBox::from_id_source("component_selector")
            .selected_text(&*selected_component)
            .show_ui(ui, |ui| {
                for name in &names {
                    ui.selectable_value(selected_component, name.clone(), name);
                }
            });
        
        // Add component button
        if ui.button("Add Component").clicked() && !selected_component.is_empty() {
            if let Some(adder) = engine.component_adders.get(selected_component).cloned() {
                adder(engine, entity);
            } else if engine.generated_components.contains(selected_component) {
                engine.add_generated_component(entity, selected_component);
            }
        }
    }
    
    /// Draw inspector UI for multiple entities
    pub fn draw_multi_entity_ui(&self, ui: &mut egui::Ui, entities: &[Entity]) {
        ui.label(format!("Multiple entities selected ({})", entities.len()));
        ui.label("Multi-edit not yet supported");
        
        // TODO: Implement multi-entity editing
        // This would show common components and allow batch editing
    }
    
    /// Draw inspector UI when no entity is selected
    pub fn draw_no_selection_ui(&self, ui: &mut egui::Ui) {
        ui.label("No entity selected");
        ui.label("Click on an object in the scene to select it");
        
        ui.separator();
        
        // Show some helpful information
        ui.label("Inspector Help:");
        ui.label("• Select entities to edit their components");
        ui.label("• Use the gizmos to transform objects");
        ui.label("• Add components to extend functionality");
    }
}

impl Default for InspectorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for InspectorPlugin {
    fn name(&self) -> &'static str {
        "inspector"
    }
    
    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if self.initialized {
            return Ok(());
        }
        
        println!("Initializing Inspector Plugin...");
        
        // Initialize inspector-specific functionality
        self.initialized = true;
        
        Ok(())
    }
    
    fn update(&mut self, _engine: &mut Engine, _delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Update inspector logic
        Ok(())
    }
    
    fn render(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Inspector rendering is handled by the main window
        Ok(())
    }
    
    fn cleanup(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        println!("Cleaning up Inspector Plugin...");
        self.initialized = false;
        
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Helper functions for drawing common component types
pub mod component_ui {
    use egui::Ui;
    
    /// Draw a float slider
    pub fn float_slider(ui: &mut Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::Slider::new(value, range)).changed()
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
    
    /// Draw a checkbox
    pub fn checkbox(ui: &mut Ui, label: &str, checked: &mut bool) -> bool {
        ui.checkbox(checked, label).changed()
    }
    
    /// Draw a text input
    pub fn text_input(ui: &mut Ui, label: &str, text: &mut String) -> bool {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.text_edit_singleline(text).changed()
        }).inner
    }
}
