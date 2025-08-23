//! Editor Demo
//! 
//! This example demonstrates the new application framework with the editor plugin.
//! It shows how to create an application that includes the full editor interface.

use vetrace_engine::app::{app, App, InputEvent};
use vetrace_engine::engine::engine::Engine;
use sdl2::keyboard::Keycode;

/// Demo application with editor
struct EditorDemoApp {
    scene_loaded: bool,
}

impl EditorDemoApp {
    fn new() -> Self {
        Self {
            scene_loaded: false,
        }
    }
}

impl App for EditorDemoApp {
    fn setup(&mut self, engine: &mut Engine) {
        println!("🎨 Editor Demo App Setup");
        println!("Engine initialized with editor plugin!");
        
        // Create a demo scene for editing
        self.create_demo_scene(engine);
        self.scene_loaded = true;
        
        println!("✅ Demo scene created - ready for editing!");
        println!();
        println!("🎮 Editor Controls:");
        println!("   • Left Panel: Scene hierarchy and engine controls");
        println!("   • Right Panel: Component inspector and properties");
        println!("   • Bottom Panel: File explorer");
        println!("   • Top Panel: Gizmo controls and file operations");
        println!("   • Click objects to select them");
        println!("   • Use gizmos to transform selected objects");
        println!("   • Toggle Sandbox Window for object creation");
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        // Application-specific update logic
        // The editor plugin handles its own updates automatically
        
        // Example: Animate objects in the scene
        if self.scene_loaded && !engine.paused {
            // Rotate the first object slowly
            if let Some(obj) = engine.scene.objects.get_mut(0) {
                obj.orientation[1] += delta_time * 0.5; // Slow rotation
            }
            
            // Bob the second object up and down
            if engine.scene.objects.len() > 1 {
                if let Some(obj) = engine.scene.objects.get_mut(1) {
                    let time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f32();
                    obj.position[1] = (time * 2.0).sin() * 0.5;
                }
            }
        }
    }
    
    fn render(&mut self, engine: &mut Engine) {
        // Custom rendering logic
        // The editor plugin handles its own rendering automatically
    }
    
    fn cleanup(&mut self, engine: &mut Engine) {
        println!("🧹 Editor Demo App Cleanup");
    }
    
    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {
        println!("📐 Editor window resized to {}x{}", width, height);
    }
    
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {
        // Handle application-specific input
        // The editor plugin handles its own input automatically
        
        match event {
            InputEvent::KeyPressed { key } => {
                match *key {
                    Keycode::F1 => {
                        println!("🆘 Help:");
                        println!("   F1 - Show this help");
                        println!("   F2 - Create demo scene");
                        println!("   F3 - Clear scene");
                        println!("   F5 - Save scene");
                        println!("   F9 - Load scene");
                    }
                    Keycode::F2 => {
                        println!("🎬 Creating new demo scene...");
                        engine.clear_scene();
                        self.create_demo_scene(engine);
                    }
                    Keycode::F3 => {
                        println!("🗑️  Clearing scene...");
                        engine.clear_scene();
                        self.scene_loaded = false;
                    }
                    Keycode::F5 => {
                        println!("💾 Save scene functionality would go here");
                        // The editor plugin handles file operations through its UI
                    }
                    Keycode::F9 => {
                        println!("📂 Load scene functionality would go here");
                        // The editor plugin handles file operations through its UI
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl EditorDemoApp {
    fn create_demo_scene(&mut self, engine: &mut Engine) {
        // Create a variety of objects to demonstrate the editor
        
        // Sphere 1 - Orange metallic
        let mut sphere1 = vetrace_engine::scene::object::Object::default();
        sphere1.position = [-2.0, 0.0, 0.0];
        sphere1.radius = 1.0;
        sphere1.color = [1.0, 0.5, 0.1]; // Orange
        sphere1.metallic = 0.9;
        sphere1.roughness = 0.1;
        engine.spawn_object(sphere1);
        
        // Cube - Green plastic
        let mut cube = vetrace_engine::scene::object::Object::default();
        cube.position = [0.0, 1.0, 0.0];
        cube.size = [1.5, 1.5, 1.5];
        cube.is_cube = true;
        cube.color = [0.2, 0.8, 0.3]; // Green
        cube.metallic = 0.0;
        cube.roughness = 0.8;
        engine.spawn_object(cube);
        
        // Sphere 2 - Blue emissive
        let mut sphere2 = vetrace_engine::scene::object::Object::default();
        sphere2.position = [2.0, 0.0, 0.0];
        sphere2.radius = 0.8;
        sphere2.color = [0.2, 0.4, 1.0]; // Blue
        sphere2.metallic = 0.0;
        sphere2.roughness = 0.3;
        sphere2.emission_strength = 2.0;
        engine.spawn_object(sphere2);
        
        // Small red sphere
        let mut small_sphere = vetrace_engine::scene::object::Object::default();
        small_sphere.position = [0.0, -1.5, 1.0];
        small_sphere.radius = 0.5;
        small_sphere.color = [1.0, 0.2, 0.2]; // Red
        small_sphere.metallic = 0.5;
        small_sphere.roughness = 0.5;
        engine.spawn_object(small_sphere);
        
        // Ground plane (large flat cube)
        let mut ground = vetrace_engine::scene::object::Object::default();
        ground.position = [0.0, -3.0, 0.0];
        ground.size = [10.0, 0.1, 10.0];
        ground.is_cube = true;
        ground.color = [0.7, 0.7, 0.7]; // Gray
        ground.metallic = 0.0;
        ground.roughness = 0.9;
        engine.spawn_object(ground);
        
        println!("🎭 Created demo scene with {} objects:", engine.scene.objects.len());
        println!("   • Orange metallic sphere (left)");
        println!("   • Green plastic cube (center, animated)");
        println!("   • Blue emissive sphere (right)");
        println!("   • Small red sphere (front)");
        println!("   • Gray ground plane");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎨 Vetrace Engine - Editor Demo");
    println!("===============================");
    println!();
    println!("This demo shows the complete editor interface using the plugin system.");
    println!("The editor provides a full-featured interface for scene editing.");
    println!();
    
    // Note: This example would require the vetrace_editor crate to be added as a dependency
    // For now, we'll show how it would work conceptually
    
    println!("⚠️  Note: This example requires the vetrace_editor crate to be compiled.");
    println!("   To use the editor plugin, add this to your Cargo.toml:");
    println!("   vetrace_editor = {{ path = \"../vetrace_editor\" }}");
    println!();
    println!("   Then use it like this:");
    println!();
    println!("   app()");
    println!("       .with_title(\"My Game with Editor\")");
    println!("       .add_plugin(vetrace_editor::editor())");
    println!("       .run(MyApp::new())");
    println!();
    
    // For now, run without the editor plugin to demonstrate the framework
    app()
        .with_title("Vetrace Engine - Editor Demo (Framework Only)")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(EditorDemoApp::new())?;
    
    println!();
    println!("👋 Editor demo completed!");
    
    Ok(())
}

// Example of how the editor plugin would be used (commented out since vetrace_editor isn't compiled yet)
/*
use vetrace_editor;

fn main_with_editor() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("Vetrace Engine - Full Editor")
        .with_size(1280, 720)
        .with_vsync(true)
        .add_plugin(vetrace_editor::editor())
        .run(EditorDemoApp::new())
}
*/
