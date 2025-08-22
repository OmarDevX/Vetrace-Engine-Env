//! Sandbox Window Implementation
//! 
//! This window provides tools for creating and manipulating objects in the scene.

use vetrace_engine::{engine::engine::Engine, scene::object::Object};
use egui::{Context, Slider, TextEdit, Ui};

use crate::EditorWindow;

/// Sandbox window for object creation and scene manipulation
#[derive(Clone)]
pub struct SandboxWindow {
    pub new_object: Object,
    pub skycolor: [f32; 3],
    pub is_fisheye: bool,
    pub new_object_position_str: [String; 3],
    pub new_object_size_str: [String; 3],
}

impl SandboxWindow {
    /// Create a new sandbox window
    pub fn new() -> Self {
        Self {
            new_object: Object::default(),
            is_fisheye: false,
            skycolor: [30.0, 255.0, 255.0],
            new_object_position_str: ["0.0".to_owned(), "0.0".to_owned(), "0.0".to_owned()],
            new_object_size_str: ["1.0".to_owned(), "1.0".to_owned(), "1.0".to_owned()],
        }
    }
    
    /// Main UI rendering function
    pub fn ui(&mut self, ctx: &Context, ui: &mut Ui, engine: &mut Engine) {
        ui.heading("Sandbox Window");
        ui.separator();

        // Rendering options
        ui.checkbox(&mut self.is_fisheye, "Enable Fisheye");

        // Sky color controls
        ui.horizontal(|ui| {
            ui.label("Sky Color");
            for i in 0..3 {
                ui.add(
                    Slider::new(&mut self.skycolor[i], 0.0..=255.0).text(["R", "G", "B"][i]),
                );
            }
        });

        ui.separator();

        // Object creation section
        ui.collapsing("Create New Object", |ui| {
            // Position controls
            ui.horizontal(|ui| {
                ui.label("Position:");
                for i in 0..3 {
                    ui.add(
                        TextEdit::singleline(&mut self.new_object_position_str[i])
                            .desired_width(60.0),
                    );
                }
            });

            // Size controls (for cubes)
            if self.new_object.is_cube {
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    for i in 0..3 {
                        ui.add(
                            TextEdit::singleline(&mut self.new_object_size_str[i])
                                .desired_width(60.0),
                        );
                    }
                });
            }

            // Object properties
            ui.add(Slider::new(&mut self.new_object.radius, 0.1..=100.0).text("Radius"));
            ui.checkbox(&mut self.new_object.is_cube, "Is Cube");

            // Material properties
            ui.separator();
            ui.label("Material Properties:");
            
            ui.horizontal(|ui| {
                ui.label("Color:");
                for i in 0..3 {
                    ui.add(Slider::new(&mut self.new_object.color[i], 0.0..=1.0).text(["R", "G", "B"][i]));
                }
            });
            
            ui.add(Slider::new(&mut self.new_object.roughness, 0.0..=1.0).text("Roughness"));
            ui.add(Slider::new(&mut self.new_object.emission, 0.0..=10.0).text("Emission"));

            // Create object button
            if ui.button("Add Object").clicked() {
                let mut new_object = self.new_object.clone();
                
                // Parse position
                for i in 0..3 {
                    new_object.position[i] = self.new_object_position_str[i]
                        .parse::<f32>()
                        .unwrap_or(0.0);
                    
                    // Parse size for cubes
                    if new_object.is_cube {
                        new_object.size[i] = self.new_object_size_str[i]
                            .parse::<f32>()
                            .unwrap_or(1.0)
                            .max(0.1);
                    }
                }
                
                // Spawn the object
                engine.spawn_object(new_object);
            }
        });

        ui.separator();

        // Scene management
        ui.collapsing("Scene Management", |ui| {
            if ui.button("Clear Scene").clicked() {
                engine.clear_scene();
            }
            
            if ui.button("Reset Camera").clicked() {
                // Reset camera to default position
                // This would need to be implemented in the engine
            }
            
            ui.separator();
            
            // Scene statistics
            ui.label("Scene Statistics:");
            ui.label(format!("Objects: {}", engine.scene.objects.len()));
            ui.label(format!("Entities: {}", engine.world.entities().len()));
        });

        ui.separator();

        // Lighting controls
        ui.collapsing("Lighting", |ui| {
            ui.label("Ambient Light:");
            ui.horizontal(|ui| {
                // This would control ambient lighting if implemented
                ui.label("Intensity:");
                let mut ambient_intensity = 0.1f32;
                ui.add(Slider::new(&mut ambient_intensity, 0.0..=1.0));
            });
            
            ui.separator();
            
            if ui.button("Add Point Light").clicked() {
                // Add a point light to the scene
                // This would need to be implemented
            }
            
            if ui.button("Add Directional Light").clicked() {
                // Add a directional light to the scene
                // This would need to be implemented
            }
        });

        ui.separator();

        // Performance monitoring
        ui.collapsing("Performance", |ui| {
            ui.label("Frame Time: N/A ms");
            ui.label("FPS: N/A");
            ui.label("Draw Calls: N/A");
            ui.label("Triangles: N/A");
            
            ui.separator();
            
            if ui.button("Force GC").clicked() {
                // Force garbage collection if applicable
            }
        });
    }
}

impl Default for SandboxWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorWindow for SandboxWindow {
    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing Sandbox Window...");
        Ok(())
    }
    
    fn update(&mut self, _engine: &mut Engine, _delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Update sandbox window logic
        Ok(())
    }
    
    fn render(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Rendering is handled by the ui() method when called from MainWindow
        Ok(())
    }
    
    fn cleanup(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("Cleaning up Sandbox Window...");
        Ok(())
    }
}
