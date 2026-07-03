//! App Framework Demo with Editor
//!
//! This example demonstrates the new application framework with the editor plugin.
//! It shows how to create a simple application that includes the full editor interface.
use vetrace_engine::materials::{PbrMaterial, MATERIAL_TAG_NEEDS_ACCURATE_REFLECTION};
use sdl2::keyboard::Keycode;
use vetrace_engine::app::{app, plugin::Plugin, App, InputEvent};
use vetrace_engine::engine::engine::Engine;
// Import the editor plugin
extern crate vetrace_editor;
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
    last_mouse_pos: (i32, i32),
}

impl DemoApp {
    fn new() -> Self {
        Self {
            last_mouse_pos: (0, 0),
        }
    }
}

impl App for DemoApp {
    fn setup(&mut self, engine: &mut Engine) {
        println!("🚀 Demo App Setup");
        println!("Engine initialized with app framework!");
        // Use a moderate sky color for a clean starting scene
        engine.sky_color = [30.0, 255.0, 255.0];
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
        // Enable post-processing with neutral exposure so lighting stays balanced
        engine
            .world
            .insert(camera_entity, PostProcessing::default());

        // Add directional light for proper lighting
        engine.world.insert(
            camera_entity,
            DirectionalLight {
                direction: [-1.0, -1.0, -1.0], // Light coming from upper-left
                color: [255.0, 255.0, 255.0],  // White light
                intensity: 1.0,                // Full intensity
                ..Default::default()
            },
        );

        // Seed the demo with visible geometry directly in front of the default
        // camera. Without scene objects the renderer has nothing but the
        // background/editor UI to draw, which makes the example look like it is
        // stuck on a black screen even though the app loop is running.
        use vetrace_engine::scene::object::Object;

        let mut cube = Object::default();
        cube.position = [4.0, 0.0, 0.0];
        cube.size = [1.5, 1.5, 1.5];
        cube.is_cube = true;
        cube.color = [0.15, 0.75, 1.0];
        cube.roughness = 0.35;
        cube.emission = 0.25;
        engine.spawn_object(cube);
        let mut ground = Object::default();
        ground.position = [4.0, -1.25, 0.0];
        ground.size = [6.0, 0.15, 6.0];
        ground.is_cube = true;

        // Object still needs basic values because Renderable is created from Object.
        ground.color = [1.0, 1.0, 1.0];
        ground.roughness = 0.0;
        ground.emission = 0.0;

        // Strong dielectric/specular fallback too, useful if metallic path is not used somewhere.
        ground.specular_f0 = [1.0, 1.0, 1.0];
        ground.ior = 10.0;

        engine.spawn_object(ground);

        // Attach real PBR material to the spawned ground entity.
        // Object itself has no metallic field, but PbrMaterial does.
        let ground_object_id = (engine.scene.objects.len() - 1) as u32;


        println!(
            "🎭 App framework demo scene initialized with {} visible objects",
            engine.scene.objects.len()
        );
    }

    fn render(&mut self, engine: &mut Engine) {
        engine.render_frame();
    }
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {
        match event {
            InputEvent::KeyPressed { key } => match *key {
                Keycode::Space => {
                    println!("⏸️  Space pressed - Pause/Resume not implemented in app framework");
                }
                Keycode::R => {
                    println!("🔄 R pressed - Restart not implemented in app framework");
                }
                Keycode::C => {
                    println!("🧹 C pressed - Clear scene");
                    engine.scene.objects.clear();
                    engine.scene.bvh_dirty = true;
                }
                Keycode::Escape => {
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
    // Create and run the application with demo plugin and editor
    app()
        .with_title("Vetrace Engine - App Framework Demo with Editor")
        .with_size(720, 720)
        .with_vsync(false)
        .add_plugin(DemoPlugin::new()) // Add the demo plugin
        .add_plugin(EditorPlugin::new()) // Add the editor plugin
        .run(DemoApp::new())?;
    Ok(())
}
