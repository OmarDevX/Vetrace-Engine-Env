//! Plugin System for Vetrace Engine
//! 
//! This module provides a flexible plugin architecture that allows features
//! to be added to the engine without tight coupling.

use crate::engine::engine::Engine;
use std::any::Any;
use std::collections::HashMap;

/// Trait that all plugins must implement
pub trait Plugin: 'static + Send + Sync {
    /// Unique name for this plugin
    fn name(&self) -> &'static str;
    
    /// Called when the plugin is first registered
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Called every frame during the update phase
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Called every frame during the render phase
    fn render(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Called when the plugin is being unloaded
    fn cleanup(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    /// Get plugin dependencies (other plugins that must be loaded first)
    fn dependencies(&self) -> Vec<&'static str> {
        Vec::new()
    }
    
    /// Check if this plugin is compatible with the current engine version
    fn is_compatible(&self, engine_version: &str) -> bool {
        true // Default to compatible
    }

    /// Render UI for this plugin (optional)
    fn render_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        Ok(()) // Default implementation does nothing
    }

    /// Allow downcasting to concrete plugin types
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Manages all registered plugins
pub struct PluginManager {
    plugins: HashMap<String, Box<dyn Plugin>>,
    plugin_order: Vec<String>,
    initialized: bool,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_order: Vec::new(),
            initialized: false,
        }
    }
    
    /// Register a plugin
    pub fn register_plugin<P: Plugin>(&mut self, plugin: P) {
        let name = plugin.name().to_string();
        
        if self.plugins.contains_key(&name) {
            eprintln!("Warning: Plugin '{}' is already registered, replacing...", name);
        }
        
        self.plugins.insert(name.clone(), Box::new(plugin));
        
        // Add to order if not already present
        if !self.plugin_order.contains(&name) {
            self.plugin_order.push(name);
        }
        
        // If already initialized, we need to re-sort for dependencies
        if self.initialized {
            self.sort_plugins_by_dependencies();
        }
    }

    /// Register a boxed plugin
    pub fn register_plugin_boxed(&mut self, plugin: Box<dyn Plugin>) {
        let name = plugin.name().to_string();

        if self.plugins.contains_key(&name) {
            eprintln!("Warning: Plugin '{}' is already registered, replacing...", name);
        }

        self.plugins.insert(name.clone(), plugin);

        // Add to order if not already present
        if !self.plugin_order.contains(&name) {
            self.plugin_order.push(name);
        }

        // If already initialized, we need to re-sort for dependencies
        if self.initialized {
            self.sort_plugins_by_dependencies();
        }
    }
    
    /// Unregister a plugin
    pub fn unregister_plugin(&mut self, name: &str) -> Option<Box<dyn Plugin>> {
        self.plugin_order.retain(|n| n != name);
        self.plugins.remove(name)
    }
    
    /// Get a reference to a plugin
    pub fn get_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.plugins.get(name).map(|p| p.as_ref())
    }
    
    /// Get a mutable reference to a plugin
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut dyn Plugin> {
        self.plugins.get_mut(name).map(|p| p.as_mut())
    }
    
    /// Check if a plugin is registered
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }
    
    /// Get list of all registered plugin names
    pub fn plugin_names(&self) -> Vec<&str> {
        self.plugin_order.iter().map(|s| s.as_str()).collect()
    }
    
    /// Initialize all plugins in dependency order
    pub fn initialize_plugins(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        if self.initialized {
            return Ok(());
        }
        
        // Sort plugins by dependencies
        self.sort_plugins_by_dependencies();
        
        // Initialize each plugin in order
        for name in &self.plugin_order.clone() {
            if let Some(plugin) = self.plugins.get_mut(name) {
                println!("Initializing plugin: {}", name);
                plugin.initialize(engine)?;
            }
        }
        
        self.initialized = true;
        Ok(())
    }
    
    /// Update all plugins
    pub fn update_plugins(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        for name in &self.plugin_order.clone() {
            if let Some(plugin) = self.plugins.get_mut(name) {
                plugin.update(engine, delta_time)?;
            }
        }
        Ok(())
    }
    
    /// Render all plugins
    pub fn render_plugins(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        for name in &self.plugin_order.clone() {
            if let Some(plugin) = self.plugins.get_mut(name) {
                plugin.render(engine)?;
            }
        }
        Ok(())
    }

    /// Render plugin UIs (specifically for editor plugins that need EGUI context)
    pub fn render_plugin_uis(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Render UI for all plugins that support it
        for (name, plugin) in &mut self.plugins {
            if let Err(e) = plugin.render_ui(ctx, engine) {
                println!("⚠️ Error rendering UI for plugin '{}': {}", name, e);
            }
        }
        Ok(())
    }

    /// Set up editor UI callback if editor plugin is present
    pub fn setup_editor_ui_callback(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we have an editor plugin that implements EditorUIRenderer
        if self.plugins.contains_key("vetrace_editor") {
            println!("🎨 Setting up editor UI callback for vetrace_editor plugin");

            // Set up a callback that will call the editor plugin's render method
            // We need to store a reference to the plugin manager to access the editor plugin
            engine.set_editor_ui_callback(|ctx, engine| {
                // This is a placeholder - the actual rendering will be done in render_plugin_uis
                Ok(())
            });
        }
        Ok(())
    }

    /// Render the complete editor UI with all components
    fn render_full_editor_ui(ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Main Editor Window - Scene Hierarchy
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

                    if ui.selectable_label(false, &object_name).clicked() {
                        println!("🎯 Selected object: {}", object_name);
                    }

                    // Show object details
                    ui.indent(format!("obj_{}", i), |ui| {
                        ui.label(format!("Position: [{:.2}, {:.2}, {:.2}]", obj.position[0], obj.position[1], obj.position[2]));
                        ui.label(format!("Radius: {:.2}", obj.radius));
                        ui.label(format!("Material: {}", obj.material_index));
                    });
                }
            });

        // Inspector Window
        egui::Window::new("🔍 Inspector")
            .default_open(true)
            .resizable(true)
            .default_size([300.0, 500.0])
            .show(ctx, |ui| {
                ui.label("🎛️ Object Properties");
                ui.separator();

                ui.label("Select an object to inspect its properties");
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

        // Transform Gizmos Window
        egui::Window::new("🎯 Transform Gizmos")
            .default_open(true)
            .resizable(true)
            .default_size([250.0, 200.0])
            .show(ctx, |ui| {
                ui.label("🔧 Transform Tools");
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("📍 Translate").clicked() {
                        println!("🎯 Translate mode activated");
                    }
                    if ui.button("🔄 Rotate").clicked() {
                        println!("🎯 Rotate mode activated");
                    }
                    if ui.button("📏 Scale").clicked() {
                        println!("🎯 Scale mode activated");
                    }
                });

                ui.separator();
                ui.label("Gizmo Settings:");
                ui.checkbox(&mut true, "Show gizmos");
                ui.checkbox(&mut false, "Local space");
                ui.checkbox(&mut true, "Snap to grid");
            });

        // Materials Window
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

        // Performance Monitor
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

        Ok(())
    }
    
    /// Cleanup all plugins
    pub fn cleanup_plugins(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Cleanup in reverse order
        for name in self.plugin_order.iter().rev() {
            if let Some(plugin) = self.plugins.get_mut(name) {
                println!("Cleaning up plugin: {}", name);
                plugin.cleanup(engine)?;
            }
        }
        
        self.initialized = false;
        Ok(())
    }
    
    /// Sort plugins by their dependencies using topological sort
    fn sort_plugins_by_dependencies(&mut self) {
        let mut sorted = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();
        
        fn visit(
            name: &str,
            plugins: &HashMap<String, Box<dyn Plugin>>,
            sorted: &mut Vec<String>,
            visited: &mut std::collections::HashSet<String>,
            visiting: &mut std::collections::HashSet<String>,
        ) -> Result<(), String> {
            if visiting.contains(name) {
                return Err(format!("Circular dependency detected involving plugin: {}", name));
            }
            
            if visited.contains(name) {
                return Ok(());
            }
            
            visiting.insert(name.to_string());
            
            if let Some(plugin) = plugins.get(name) {
                for dep in plugin.dependencies() {
                    if !plugins.contains_key(dep) {
                        return Err(format!("Plugin '{}' depends on '{}' which is not registered", name, dep));
                    }
                    visit(dep, plugins, sorted, visited, visiting)?;
                }
            }
            
            visiting.remove(name);
            visited.insert(name.to_string());
            sorted.push(name.to_string());
            
            Ok(())
        }
        
        // Visit all plugins
        for name in &self.plugin_order.clone() {
            if let Err(e) = visit(name, &self.plugins, &mut sorted, &mut visited, &mut visiting) {
                eprintln!("Plugin dependency error: {}", e);
                // Fall back to original order if there's a dependency issue
                return;
            }
        }
        
        self.plugin_order = sorted;
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to help create simple plugins
#[macro_export]
macro_rules! create_plugin {
    ($name:ident, $plugin_name:expr) => {
        pub struct $name;
        
        impl $crate::app::plugin::Plugin for $name {
            fn name(&self) -> &'static str {
                $plugin_name
            }
        }
    };
    
    ($name:ident, $plugin_name:expr, $($method:ident => $body:expr),*) => {
        pub struct $name;
        
        impl $crate::app::plugin::Plugin for $name {
            fn name(&self) -> &'static str {
                $plugin_name
            }
            
            $(
                fn $method(&mut self, engine: &mut $crate::engine::engine::Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
                    $body
                }
            )*
        }
    };
}
