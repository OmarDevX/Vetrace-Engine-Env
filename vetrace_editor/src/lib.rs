//! Vetrace Editor - A comprehensive editor plugin for Vetrace Engine
//!
//! This crate provides a complete editor interface built on top of EGUI,
//! including scene editing, component inspection, asset management, and more.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::engine::ui::EditorUIRenderer;

pub mod windows;
pub mod inspector;
pub mod gizmo;
pub mod selection;
pub mod ui_components;

pub use windows::{MainWindow, SandboxWindow};
pub use inspector::InspectorPlugin;
pub use gizmo::GizmoPlugin;
pub use selection::SelectionPlugin;

/// Main editor plugin that provides the complete editor interface
pub struct EditorPlugin {
    main_window: MainWindow,
    sandbox_window: SandboxWindow,
    inspector: InspectorPlugin,
    gizmo: GizmoPlugin,
    selection: SelectionPlugin,
    initialized: bool,
    // UI state
    pub selected_object: Option<usize>,
    pub show_scene_hierarchy: bool,
    pub show_inspector: bool,
    pub show_gizmos: bool,
    pub show_materials: bool,
    pub show_performance: bool,
    pub gizmo_mode: GizmoMode,
    pub show_gizmo_settings: bool,
    pub local_space: bool,
    pub snap_to_grid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

impl EditorPlugin {
    /// Create a new editor plugin
    pub fn new() -> Self {
        Self {
            main_window: MainWindow::new(),
            sandbox_window: SandboxWindow::new(),
            inspector: InspectorPlugin::new(),
            gizmo: GizmoPlugin::new(),
            selection: SelectionPlugin::new(),
            initialized: false,
            // Initialize UI state
            selected_object: None,
            show_scene_hierarchy: true,
            show_inspector: true,
            show_gizmos: true,
            show_materials: false,
            show_performance: false,
            gizmo_mode: GizmoMode::Translate,
            show_gizmo_settings: true,
            local_space: false,
            snap_to_grid: true,
        }
    }
    
    /// Get a reference to the main window
    pub fn main_window(&self) -> &MainWindow {
        &self.main_window
    }
    
    /// Get a mutable reference to the main window
    pub fn main_window_mut(&mut self) -> &mut MainWindow {
        &mut self.main_window
    }
    
    /// Get a reference to the sandbox window
    pub fn sandbox_window(&self) -> &SandboxWindow {
        &self.sandbox_window
    }
    
    /// Get a mutable reference to the sandbox window
    pub fn sandbox_window_mut(&mut self) -> &mut SandboxWindow {
        &mut self.sandbox_window
    }
    
    /// Check if the editor wants to capture input
    pub fn wants_input(&self, engine: &Engine) -> bool {
        engine.egui_ctx.wants_pointer_input() || 
        engine.egui_ctx.wants_keyboard_input() ||
        self.main_window.gizmo_hovered
    }
    
    /// Get the blur rectangles for background effects
    pub fn blur_rects(&self) -> Vec<egui::Rect> {
        self.main_window.blur_rects()
    }

    /// Render the editor UI during an EGUI frame
    /// This method should be called from within an EGUI context
    pub fn render_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }

        // Render the full editor UI with all components
        self.render_full_editor_ui(ctx, engine)?;

        Ok(())
    }

    /// Render the complete editor UI with all components and proper state management
    fn render_full_editor_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Scene Hierarchy Window
        if self.show_scene_hierarchy {
            egui::Window::new("🌳 Scene Hierarchy")
                .default_open(true)
                .resizable(true)
                .default_size([300.0, 400.0])
                .show(ctx, |ui| {
                    ui.label("📋 Scene Objects");
                    ui.separator();

                    // List all entities in the scene
                    let entity_count = engine.world.entities().len();
                    ui.label(format!("Total entities: {}", entity_count));

                    ui.separator();

                    // Show objects from the scene
                    for (i, obj) in engine.scene.objects.iter().enumerate() {
                        let object_name = format!("Object {} ({})", i, if obj.is_cube { "Cube" } else { "Sphere" });

                        let is_selected = self.selected_object == Some(i);
                        if ui.selectable_label(is_selected, &object_name).clicked() {
                            self.selected_object = if is_selected { None } else { Some(i) };
                            println!("🎯 Selected object: {}", if is_selected { "None" } else { &object_name });
                        }

                        // Show object details if selected
                        if is_selected {
                            ui.indent(format!("obj_{}", i), |ui| {
                                ui.label(format!("Position: [{:.2}, {:.2}, {:.2}]", obj.position[0], obj.position[1], obj.position[2]));
                                ui.label(format!("Radius: {:.2}", obj.radius));
                                ui.label(format!("Material: {}", obj.material_index));
                            });
                        }
                    }
                });
        }

        // Inspector Window
        if self.show_inspector {
            egui::Window::new("🔍 Inspector")
                .default_open(true)
                .resizable(true)
                .default_size([300.0, 500.0])
                .show(ctx, |ui| {
                    ui.label("🎛️ Object Properties");
                    ui.separator();

                    if let Some(selected_idx) = self.selected_object {
                        if let Some(obj) = engine.scene.objects.get(selected_idx) {
                            ui.label(format!("Selected: Object {}", selected_idx));
                            ui.separator();

                            // Object properties (read-only for now, but could be made editable)
                            ui.label(format!("Position: [{:.2}, {:.2}, {:.2}]", obj.position[0], obj.position[1], obj.position[2]));
                            ui.label(format!("Radius: {:.2}", obj.radius));
                            ui.label(format!("Color: [{:.0}, {:.0}, {:.0}]", obj.color[0], obj.color[1], obj.color[2]));
                            ui.label(format!("Roughness: {:.2}", obj.roughness));
                            ui.label(format!("Emission: {:.2}", obj.emission));
                            ui.label(format!("Is Cube: {}", obj.is_cube));
                            ui.label(format!("Material Index: {}", obj.material_index));
                        }
                    } else {
                        ui.label("Select an object to inspect its properties");
                    }

                    ui.separator();

                    // Camera properties
                    ui.collapsing("📷 Camera", |ui| {
                        let cam_info = engine.active_camera_info();
                        ui.label(format!("Position: [{:.2}, {:.2}, {:.2}]",
                            cam_info.position.x, cam_info.position.y, cam_info.position.z));
                        ui.label(format!("Orientation: [{:.2}, {:.2}, {:.2}, {:.2}]",
                            cam_info.orientation.x, cam_info.orientation.y, cam_info.orientation.z, cam_info.orientation.w));
                        ui.label(format!("FOV: {:.1}°", cam_info.fov.to_degrees()));
                    });

                    // Scene properties
                    ui.collapsing("🌍 Scene", |ui| {
                        ui.label(format!("Objects: {}", engine.scene.objects.len()));
                        ui.label(format!("Materials: {}", engine.scene.materials.len()));
                        ui.label(format!("BVH Nodes: {}", engine.scene.bvh_nodes.len()));
                    });

                    // Rendering properties
                    ui.collapsing("🎨 Rendering", |ui| {
                        ui.label("Renderer: WGPU");
                        ui.label("Shading: PBR");
                        ui.label("Post-processing: Enabled");
                    });
                });
        }

        // Transform Gizmos Window
        if self.show_gizmos {
            egui::Window::new("🎯 Transform Gizmos")
                .default_open(true)
                .resizable(true)
                .default_size([250.0, 200.0])
                .show(ctx, |ui| {
                    ui.label("🔧 Transform Tools");
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.gizmo_mode == GizmoMode::Translate, "📍 Translate").clicked() {
                            self.gizmo_mode = GizmoMode::Translate;
                            println!("🎯 Translate mode activated");
                        }
                        if ui.selectable_label(self.gizmo_mode == GizmoMode::Rotate, "🔄 Rotate").clicked() {
                            self.gizmo_mode = GizmoMode::Rotate;
                            println!("🎯 Rotate mode activated");
                        }
                        if ui.selectable_label(self.gizmo_mode == GizmoMode::Scale, "📏 Scale").clicked() {
                            self.gizmo_mode = GizmoMode::Scale;
                            println!("🎯 Scale mode activated");
                        }
                    });

                    ui.separator();
                    ui.label("Gizmo Settings:");
                    ui.checkbox(&mut self.show_gizmo_settings, "Show gizmos");
                    ui.checkbox(&mut self.local_space, "Local space");
                    ui.checkbox(&mut self.snap_to_grid, "Snap to grid");

                    if let Some(selected_idx) = self.selected_object {
                        ui.separator();
                        ui.label(format!("Selected Object: {}", selected_idx));
                        ui.label("Use gizmos to transform the selected object");
                    }
                });
        }

        // Materials Window
        if self.show_materials {
            egui::Window::new("🎨 Materials")
                .default_open(false)
                .resizable(true)
                .default_size([300.0, 400.0])
                .show(ctx, |ui| {
                    ui.label("🎭 Material Editor");
                    ui.separator();

                    for (i, material) in engine.scene.materials.iter().enumerate() {
                        ui.collapsing(format!("Material {}", i), |ui| {
                            ui.label(format!("Base Color: [{:.2}, {:.2}, {:.2}, {:.2}]",
                                material.base_color[0], material.base_color[1],
                                material.base_color[2], material.base_color[3]));
                            ui.label(format!("Roughness: {:.2}", material.roughness));
                            ui.label(format!("Metallic: {:.2}", material.metallic));
                        });
                    }
                });
        }

        // Performance Monitor
        if self.show_performance {
            egui::Window::new("📊 Performance")
                .default_open(false)
                .resizable(true)
                .default_size([250.0, 150.0])
                .show(ctx, |ui| {
                    ui.label("⚡ Performance Metrics");
                    ui.separator();

                    ui.label("FPS: ~60");
                    ui.label("Frame Time: ~16ms");
                    ui.label("Draw Calls: 5");
                    ui.label("Triangles: 2,880");
                });
        }

        // Window visibility controls (menu bar or separate window)
        egui::Window::new("🎛️ Editor Controls")
            .default_open(true)
            .resizable(true)
            .default_size([200.0, 200.0])
            .show(ctx, |ui| {
                ui.label("🪟 Window Visibility");
                ui.separator();

                ui.checkbox(&mut self.show_scene_hierarchy, "🌳 Scene Hierarchy");
                ui.checkbox(&mut self.show_inspector, "🔍 Inspector");
                ui.checkbox(&mut self.show_gizmos, "🎯 Transform Gizmos");
                ui.checkbox(&mut self.show_materials, "🎨 Materials");
                ui.checkbox(&mut self.show_performance, "📊 Performance");

                ui.separator();
                ui.label("🎮 Editor Status");
                ui.label(format!("Selected: {}",
                    if let Some(idx) = self.selected_object {
                        format!("Object {}", idx)
                    } else {
                        "None".to_string()
                    }
                ));
                ui.label(format!("Gizmo Mode: {:?}", self.gizmo_mode));
            });

        Ok(())
    }
}

impl Default for EditorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for EditorPlugin {
    fn name(&self) -> &'static str {
        "vetrace_editor"
    }
    
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if self.initialized {
            return Ok(());
        }
        
        println!("Initializing Vetrace Editor...");
        
        // Initialize sub-plugins
        self.inspector.initialize(engine)?;
        self.gizmo.initialize(engine)?;
        self.selection.initialize(engine)?;
        
        // Initialize windows
        self.main_window.initialize(engine)?;
        self.sandbox_window.initialize(engine)?;
        
        self.initialized = true;
        println!("Vetrace Editor initialized successfully!");
        
        Ok(())
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        // Update sub-plugins
        self.inspector.update(engine, delta_time)?;
        self.gizmo.update(engine, delta_time)?;
        self.selection.update(engine, delta_time)?;
        
        // Update windows
        self.main_window.update(engine, delta_time)?;
        self.sandbox_window.update(engine, delta_time)?;
        
        Ok(())
    }
    
    fn render(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // The editor plugin's render method is called during the plugin render phase
        // The actual UI rendering happens during the EGUI frame in the engine's draw_editor_ui method
        // This method is kept for compatibility but doesn't render UI directly
        Ok(())
    }

    fn render_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Render the full editor UI with all components
        self.render_full_editor_ui(ctx, engine)
    }
    
    fn cleanup(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }
        
        println!("Cleaning up Vetrace Editor...");
        
        // Cleanup sub-plugins
        self.selection.cleanup(engine)?;
        self.gizmo.cleanup(engine)?;
        self.inspector.cleanup(engine)?;
        
        // Cleanup windows
        self.sandbox_window.cleanup(engine)?;
        self.main_window.cleanup(engine)?;
        
        self.initialized = false;
        println!("Vetrace Editor cleaned up successfully!");
        
        Ok(())
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        // Editor depends on core engine systems
        vec![]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl EditorUIRenderer for EditorPlugin {
    fn render_editor_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Call the existing render_ui method
        self.render_ui(ctx, engine)
    }
}

/// Convenience function to create and configure the editor plugin
pub fn editor() -> EditorPlugin {
    EditorPlugin::new()
}

/// Trait for editor windows
pub trait EditorWindow {
    /// Initialize the window
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Update the window logic
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Render the window UI
    fn render(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Cleanup the window
    fn cleanup(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// Trait for editor tools
pub trait EditorTool {
    /// Tool name
    fn name(&self) -> &'static str;
    
    /// Initialize the tool
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Update the tool
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Render tool UI
    fn render(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Handle tool input
    fn handle_input(&mut self, engine: &mut Engine, event: &EditorInputEvent) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(false) // Return true if input was consumed
    }
    
    /// Cleanup the tool
    fn cleanup(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// Editor-specific input events
#[derive(Debug, Clone)]
pub enum EditorInputEvent {
    KeyPressed { key: String },
    KeyReleased { key: String },
    MousePressed { button: MouseButton, x: i32, y: i32 },
    MouseReleased { button: MouseButton, x: i32, y: i32 },
    MouseMoved { x: i32, y: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}
