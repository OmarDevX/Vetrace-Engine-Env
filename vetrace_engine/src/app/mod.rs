//! Application Framework for Vetrace Engine
//!
//! This module provides a clean application framework similar to Bevy's App system,
//! allowing users to create applications without dealing with engine internals.

use crate::ecs::behaviour::Behaviour;
use crate::ecs::World;
use crate::engine::engine::Engine;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use sdl2::keyboard::Keycode;

pub mod events;
pub mod plugin;

pub use events::{Event, EventBus};
pub use plugin::{Plugin, PluginManager};

/// Main application trait that users implement to create their applications
pub trait App: 'static {
    /// Called once when the application starts
    fn setup(&mut self, engine: &mut Engine) {}

    /// Called every frame during the update phase
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}

    /// Called every frame during the render phase
    fn render(&mut self, engine: &mut Engine) {}

    /// Called when the application is shutting down
    fn cleanup(&mut self, engine: &mut Engine) {}

    /// Called when the window is resized
    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {}

    /// Called for input events
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {}
}

/// Input events that can be handled by applications
#[derive(Debug, Clone, Copy)]
pub enum InputEvent {
    KeyPressed { key: Keycode },
    KeyReleased { key: Keycode },
    MousePressed { button: MouseButton, x: i32, y: i32 },
    MouseReleased { button: MouseButton, x: i32, y: i32 },
    MouseMoved { x: i32, y: i32 },
    WindowResized { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Application builder that provides a fluent API for configuring applications
pub struct AppBuilder {
    plugins: Vec<Box<dyn Plugin>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
    window_title: String,
    window_size: (u32, u32),
    render_scale: f32,
    fsr: Option<f32>,
    vsync: bool,
    event_bus: EventBus,
}

impl Default for AppBuilder {
    fn default() -> Self {
        Self {
            plugins: Vec::new(),
            resources: HashMap::new(),
            window_title: "Vetrace Engine Application".to_string(),
            window_size: (1280, 720),
            render_scale: 1.0,
            fsr: None,
            vsync: true,
            event_bus: EventBus::new(),
        }
    }
}

impl AppBuilder {
    /// Create a new application builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the window title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = title.into();
        self
    }

    /// Set the window size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.window_size = (width, height);
        self
    }

    /// Set internal rendering scale (0.1-1.0)
    pub fn with_render_scale(mut self, scale: f32) -> Self {
        self.render_scale = scale;
        self
    }

    /// Enable AMD FSR upscaling with sharpness
    pub fn with_fsr(mut self, sharpness: f32) -> Self {
        self.fsr = Some(sharpness);
        self
    }

    /// Enable or disable vsync
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Add a plugin to the application
    pub fn add_plugin<P: Plugin>(mut self, plugin: P) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    /// Add a resource that can be accessed by systems and plugins
    pub fn add_resource<T: 'static>(mut self, resource: T) -> Self {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
        self
    }

    /// Configure the event bus
    pub fn with_event_bus(mut self, mut configure: impl FnMut(&mut EventBus)) -> Self {
        configure(&mut self.event_bus);
        self
    }

    /// Build and run the application
    pub fn run<A: App>(self, mut app: A) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize the engine with the configured settings
        let mut engine = Engine::new(false); // Default to 3D mode

        // Configure window
        engine
            .window
            .window
            .set_title(&self.window_title)
            .expect("failed to set window title");
        engine
            .window
            .window
            .set_size(self.window_size.0, self.window_size.1);
        engine.set_render_scale(self.render_scale);
        if let Some(s) = self.fsr {
            engine.enable_fsr(s);
        }

        // Create plugin manager and register plugins
        let mut plugin_manager = PluginManager::new();
        for plugin in self.plugins {
            plugin_manager.register_plugin_boxed(plugin);
        }

        // Initialize plugins
        plugin_manager.initialize_plugins(&mut engine)?;

        // Register a callback that renders plugin UIs
        let plugin_manager_ptr = &mut plugin_manager as *mut PluginManager;
        engine.add_ui_callback(move |ctx, engine| {
            // Safely access the plugin manager
            let plugin_manager = unsafe { &mut *plugin_manager_ptr };
            plugin_manager.render_plugin_uis(ctx, engine)
        });

        // Add resources to the world
        for (type_id, resource) in self.resources {
            // Store resources in the world (we'll need to extend World to support this)
            // For now, we'll store them in the engine
        }

        // Call app setup
        app.setup(&mut engine);

        // Start engine behaviours and component behaviours so systems like
        // post-processing run when using the app framework
        let mut behaviours = std::mem::take(&mut engine.behaviours);
        for b in behaviours.iter_mut() {
            b.start(&mut engine);
        }
        engine.behaviours = behaviours;
        engine.start_component_behaviours();

        // Main application loop
        let mut last_time = std::time::Instant::now();

        while engine.running {
            let current_time = std::time::Instant::now();
            let delta_time = (current_time - last_time).as_secs_f32();
            last_time = current_time;

            // Handle SDL events manually since engine doesn't have a simple update method
            // Clear EGUI events from previous frame
            engine.egui_events.clear();

            // Get mouse position for EGUI event conversion
            let mouse_state = sdl2::mouse::MouseState::new(&engine.window.event_pump);
            let mouse_pos = egui::Pos2::new(mouse_state.x() as f32, mouse_state.y() as f32);

            let events: Vec<_> = engine.window.poll_iter().collect();
            let mut input_events = Vec::new();
            for event in events {
                engine.input.update(&event);

                // Convert SDL events to EGUI events
                if let Some(egui_event) =
                    crate::engine::engine::sdl_event_to_egui_event(&event, mouse_pos)
                {
                    engine.egui_events.push(egui_event);
                }

                match event {
                    sdl2::event::Event::Quit { .. } => {
                        engine.running = false;
                    }
                    sdl2::event::Event::Window { win_event, .. } => match win_event {
                        sdl2::event::WindowEvent::Resized(w, h)
                        | sdl2::event::WindowEvent::SizeChanged(w, h) => {
                            engine.window.resize(w, h);
                            engine.renderer.resize(w, h);
                            #[cfg(feature = "use_epi")]
                            engine
                                .egui_renderer
                                .update_screen_rect((w as u32, h as u32));
                            app.on_resize(&mut engine, w as u32, h as u32);
                            input_events.push(InputEvent::WindowResized {
                                width: w as u32,
                                height: h as u32,
                            });
                        }
                        _ => {}
                    },
                    sdl2::event::Event::KeyDown {
                        keycode: Some(k), ..
                    } => {
                        if k == Keycode::Escape {
                            engine.running = false;
                        }
                        input_events.push(InputEvent::KeyPressed { key: k });
                    }
                    sdl2::event::Event::KeyUp {
                        keycode: Some(k), ..
                    } => {
                        input_events.push(InputEvent::KeyReleased { key: k });
                    }
                    sdl2::event::Event::MouseMotion { x, y, .. } => {
                        input_events.push(InputEvent::MouseMoved { x, y });
                    }
                    sdl2::event::Event::MouseButtonDown {
                        mouse_btn, x, y, ..
                    } => {
                        let button = match mouse_btn {
                            sdl2::mouse::MouseButton::Left => Some(MouseButton::Left),
                            sdl2::mouse::MouseButton::Right => Some(MouseButton::Right),
                            sdl2::mouse::MouseButton::Middle => Some(MouseButton::Middle),
                            _ => None,
                        };
                        if let Some(button) = button {
                            input_events.push(InputEvent::MousePressed { button, x, y });
                        }
                    }
                    sdl2::event::Event::MouseButtonUp {
                        mouse_btn, x, y, ..
                    } => {
                        let button = match mouse_btn {
                            sdl2::mouse::MouseButton::Left => Some(MouseButton::Left),
                            sdl2::mouse::MouseButton::Right => Some(MouseButton::Right),
                            sdl2::mouse::MouseButton::Middle => Some(MouseButton::Middle),
                            _ => None,
                        };
                        if let Some(button) = button {
                            input_events.push(InputEvent::MouseReleased { button, x, y });
                        }
                    }
                    _ => {}
                }
            }

            for event in &input_events {
                app.on_input(&mut engine, event);
            }

            // Update plugins
            plugin_manager.update_plugins(&mut engine, delta_time)?;

            // Update engine behaviours and component behaviours so component
            // changes (like PostProcessing) take effect
            engine.update_component_behaviours(delta_time);
            let mut behaviours = std::mem::take(&mut engine.behaviours);
            for b in behaviours.iter_mut() {
                b.update(&mut engine, delta_time);
            }
            engine.behaviours = behaviours;

            // Update application
            app.update(&mut engine, delta_time);

            // Let the app handle rendering (which should call engine.render_frame())
            app.render(&mut engine);
        }

        // Cleanup
        app.cleanup(&mut engine);
        plugin_manager.cleanup_plugins(&mut engine)?;

        Ok(())
    }
}

/// Convenience function to create a new application builder
pub fn app() -> AppBuilder {
    AppBuilder::new()
}