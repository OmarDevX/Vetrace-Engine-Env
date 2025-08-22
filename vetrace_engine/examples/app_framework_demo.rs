//! App Framework Demo with Editor
//!
//! This example demonstrates the new application framework with the editor plugin.
//! It shows how to create a simple application that includes the full editor interface.

use vetrace_engine::app::{app, plugin::Plugin, App, InputEvent};
use vetrace_engine::engine::engine::Engine;

// Import the editor plugin
extern crate vetrace_editor;
use std::cell::RefCell;
use std::rc::Rc;
use vetrace_editor::EditorPlugin;

/// Simple demo plugin to show how the plugin system works
struct DemoPlugin {
    frame_count: u32,
    initialized: bool,
}

impl DemoPlugin {
    fn new() -> Self {
        Self {
            frame_count: 0,
            initialized: false,
        }
    }
}

impl Plugin for DemoPlugin {
    fn name(&self) -> &'static str {
        "demo_plugin"
    }

    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔌 Demo Plugin: Initializing...");
        self.initialized = true;
        println!("✅ Demo Plugin: Initialized successfully!");
        Ok(())
    }

    fn update(
        &mut self,
        _engine: &mut Engine,
        _delta_time: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.initialized {
            return Ok(());
        }

        self.frame_count += 1;

        // Print a message every 300 frames (about every 5 seconds at 60fps)
        if self.frame_count % 300 == 0 {
            println!(
                "🔌 Demo Plugin: Frame {} - Plugin is running!",
                self.frame_count
            );
        }

        Ok(())
    }

    fn render(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // Plugin rendering logic would go here
        Ok(())
    }

    fn cleanup(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔌 Demo Plugin: Cleaning up...");
        self.initialized = false;
        println!("✅ Demo Plugin: Cleaned up successfully!");
        Ok(())
    }

    fn dependencies(&self) -> Vec<&'static str> {
        // This plugin has no dependencies
        vec![]
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
/// Simple demo application
struct DemoApp {
    frame_count: u32,
    last_mouse_pos: (i32, i32),
}

impl DemoApp {
    fn new() -> Self {
        Self {
            frame_count: 0,
            last_mouse_pos: (0, 0),
        }
    }
}

impl App for DemoApp {
    fn setup(&mut self, engine: &mut Engine) {
        println!("🚀 Demo App Setup");
        println!("Engine initialized with app framework!");
        // Use a bright sky by default so colors appear vivid like other examples
        engine.sky_color = [255.0, 255.0, 255.0];

        // Create a simple scene using the engine's scene API
        // Add a sphere at the origin
        let mut sphere = vetrace_engine::scene::object::Object::default();
        sphere.position = [2.0, 0.0, 0.0]; // Closer to camera (positive X)
        sphere.radius = 1.0;
        sphere.color = [255.0, 128.0, 51.0]; // Orange (0-255 range)
        sphere.roughness = 0.3;
        sphere.is_cube = false; // Ensure it's a sphere
        engine.spawn_object(sphere); // Spawn so the editor can track it

        // Add a cube to the right
        let mut cube = vetrace_engine::scene::object::Object::default();
        cube.position = [2.0, 0.0, 1.5]; // Closer and to the right
        cube.size = [1.0, 1.0, 1.0];
        cube.is_cube = true;
        cube.color = [51.0, 204.0, 76.0]; // Green (0-255 range)
        cube.roughness = 0.1;
        engine.spawn_object(cube); // Spawn so the editor can track it

        // Add a third object for variety
        let mut sphere2 = vetrace_engine::scene::object::Object::default();
        sphere2.position = [2.0, 0.0, -1.5]; // Closer and to the left
        sphere2.radius = 0.8;
        sphere2.color = [204.0, 51.0, 204.0]; // Purple (0-255 range)
        sphere2.roughness = 0.5;
        sphere2.is_cube = false;
        engine.spawn_object(sphere2); // Spawn so the editor can track it

        // Set up a camera
        use vetrace_engine::components::components::{
            CameraAttachment, DirectionalLight, FreeFlightControls, PostProcessing, Transform,
        };

        // Create a camera entity
        let camera_entity = engine.world.spawn();
        engine.world.insert(
            camera_entity,
            Transform {
                position: [0.0, 0.0, 0.0], // Camera at origin looking down positive X
                orientation: [0.0, 0.0, 0.0, 1.0], // No rotation (identity quaternion)
                size: [1.0, 1.0, 1.0],
            },
        );
        engine
            .world
            .insert(camera_entity, CameraAttachment::default());
        engine.world.insert(
            camera_entity,
            FreeFlightControls {
                yaw: 0.0, // Face down +X to match scene orientation
                ..Default::default()
            },
        );
        // Enable post-processing with automatic exposure so lighting isn't overly dim
        engine.world.insert(
            camera_entity,
            PostProcessing {
                auto_exposure: true,
                ..Default::default()
            },
        );

        // Add directional light for proper lighting
        engine.world.insert(
            camera_entity,
            DirectionalLight {
                direction: [-1.0, -1.0, -1.0], // Light coming from upper-left
                color: [255.0, 255.0, 255.0],  // White light
                intensity: 1.0,                // Full intensity
            },
        );

        // Add a few more objects to make the scene more interesting for editing
        let mut sphere3 = vetrace_engine::scene::object::Object::default();
        sphere3.position = [0.0, 2.0, 0.0]; // Above the camera
        sphere3.radius = 0.6;
        sphere3.color = [255.0, 255.0, 51.0]; // Yellow
        sphere3.roughness = 0.2;
        sphere3.is_cube = false;
        engine.spawn_object(sphere3);

        let mut cube2 = vetrace_engine::scene::object::Object::default();
        cube2.position = [0.0, -1.0, 2.0]; // Below and forward
        cube2.size = [0.8, 0.8, 0.8];
        cube2.is_cube = true;
        cube2.color = [51.0, 255.0, 255.0]; // Cyan
        cube2.roughness = 0.7;
        engine.spawn_object(cube2);

        // Note: The app framework supports both primitive objects AND PBR meshes!
        // PBR meshes can be added by creating entities with MeshHandle and PbrMaterial components.
        // This demo focuses on primitive objects which work perfectly with the editor.

        println!(
            "✅ Demo scene created with {} primitive objects and camera",
            engine.scene.objects.len()
        );
        println!("🎮 Camera controls: WASD to move, mouse to look, E/Q for up/down");
        println!("🎨 Editor ready: Click objects to select, use gizmos to transform!");
    }

    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        self.frame_count += 1;

        // Print frame info occasionally
        if self.frame_count % 60 == 0 {
            println!(
                "Frame: {}, Delta: {:.3}ms",
                self.frame_count,
                delta_time * 1000.0
            );
        }
    }

    fn render(&mut self, engine: &mut Engine) {
        // The engine handles rendering automatically in the app framework
        // This includes both 3D scene rendering and EGUI rendering
        engine.render_frame();
    }

    fn cleanup(&mut self, engine: &mut Engine) {
        println!("🧹 Demo App Cleanup");
        println!("Total frames rendered: {}", self.frame_count);
    }

    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {
        println!("🖼️  Window resized to {}x{}", width, height);
    }

    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {
        match event {
            InputEvent::KeyPressed { key, .. } => match key.as_str() {
                "Space" => {
                    println!("⏸️  Space pressed - Pause/Resume not implemented in app framework");
                }
                "r" | "R" => {
                    println!("🔄 R pressed - Restart not implemented in app framework");
                }
                "c" | "C" => {
                    println!("🧹 C pressed - Clear scene");
                    engine.scene.objects.clear();
                    engine.scene.bvh_dirty = true;
                }
                "Escape" => {
                    println!("👋 ESC pressed - Exiting...");
                    std::process::exit(0);
                }
                _ => {}
            },
            InputEvent::MouseMoved { x, y } => {
                self.last_mouse_pos = (*x, *y);
            }
            InputEvent::WindowResized { width, height } => {
                println!("📐 Window resized: {}x{}", width, height);
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Vetrace Engine - App Framework Demo with Editor");
    println!("==================================================");
    println!();
    println!("This demo shows the new application framework with the editor plugin.");
    println!("It demonstrates how plugins can be added to extend engine functionality.");
    println!();
    println!("🔌 Plugin Features:");
    println!("  • Plugin initialization and cleanup");
    println!("  • Plugin update and render cycles");
    println!("  • Plugin dependency management");
    println!("  • Plugin lifecycle management");
    println!();
    println!("🎨 Editor Features:");
    println!("  • Object selection and manipulation");
    println!("  • Transform gizmos (translate, rotate, scale)");
    println!("  • Scene hierarchy view");
    println!("  • Object inspector with property editing");
    println!("  • Scene save/load functionality");
    println!();
    println!("🎮 Controls:");
    println!("  • WASD + Mouse - Free flight camera");
    println!("  • E/Q - Move up/down");
    println!("  • Left Click - Select objects");
    println!("  • Gizmos - Transform selected objects");
    println!("  • ESC - Exit application");
    println!();
    println!("📊 Watch the console for plugin messages!");
    println!();

    // Create and run the application with demo plugin and editor
    app()
        .with_title("Vetrace Engine - App Framework Demo with Editor")
        .with_size(1280, 720)
        .with_vsync(false)
        .add_plugin(DemoPlugin::new()) // Add the demo plugin
        .add_plugin(EditorPlugin::new()) // Add the editor plugin
        .run(DemoApp::new())?;

    println!();
    println!("👋 Demo with plugins completed successfully!");

    Ok(())
}
