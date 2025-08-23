//! Vetrace Editor - A comprehensive editor plugin for Vetrace Engine
//!
//! This crate provides a complete editor interface built on top of EGUI,
//! including scene editing, component inspection, asset management, and more.

use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;
use vetrace_engine::engine::ui::EditorUIRenderer;
use sdl2::keyboard::Keycode;

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

    /// Render the editor interface and update gizmos.
    fn render_full_editor_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        self.main_window.ui(ctx, &mut self.sandbox_window, engine);
        self.main_window.gizmo_hovered = self.gizmo.update_gizmo(
            engine,
            &self.main_window.selected_entities,
            self.main_window.gizmo_mode,
            self.main_window.gizmo_orientation,
        );
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
#[derive(Debug, Clone, Copy)]
pub enum EditorInputEvent {
    KeyPressed { key: Keycode },
    KeyReleased { key: Keycode },
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
