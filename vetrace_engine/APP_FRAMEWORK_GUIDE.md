# VeTrace Engine App Framework

The VeTrace Engine now includes a modern app framework that provides a clean, plugin-based architecture for building applications. This framework separates editor functionality from the core engine, making it easier to create custom applications.

## Overview

The app framework consists of several key components:

- **App Trait**: Define your application logic
- **Plugin System**: Modular functionality through plugins
- **Event System**: Type-safe event handling
- **Resource Management**: Shared application state
- **Input Handling**: Unified input event processing

## Quick Start

Here's a minimal example of using the app framework:

```rust
use vetrace_engine::app::{app, App, InputEvent};
use vetrace_engine::engine::Engine;

struct MyApp {
    frame_count: u32,
}

impl MyApp {
    fn new() -> Self {
        Self { frame_count: 0 }
    }
}

impl App for MyApp {
    fn setup(&mut self, engine: &mut Engine) {
        println!("App setup complete!");
        
        // Add some objects to the scene
        let sphere = engine.scene.add_sphere([0.0, 0.0, -5.0], 1.0);
        sphere.color = [1.0, 0.0, 0.0]; // Red sphere
    }

    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        self.frame_count += 1;
        
        if self.frame_count % 60 == 0 {
            println!("Frame: {}, Delta: {:.3}ms", self.frame_count, delta_time * 1000.0);
        }
    }

    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {
        use sdl2::keyboard::Keycode;
        match event {
            InputEvent::KeyPressed { key } => {
                println!("Key pressed: {:?}", key);
                if *key == Keycode::Escape {
                    engine.running = false;
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("My VeTrace App")
        .with_window_size(1280, 720)
        .run(MyApp::new())
}
```

## App Trait

The `App` trait defines the lifecycle of your application:

```rust
pub trait App {
    fn setup(&mut self, engine: &mut Engine) {}
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}
    fn render(&mut self, engine: &mut Engine) {}
    fn cleanup(&mut self, engine: &mut Engine) {}
    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {}
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {}
}
```

- **setup**: Called once when the app starts
- **update**: Called every frame for game logic
- **render**: Called every frame for custom rendering
- **cleanup**: Called when the app shuts down
- **on_resize**: Called when the window is resized
- **on_input**: Called for input events

## Plugin System

Plugins provide modular functionality that can be shared between applications:

```rust
use vetrace_engine::app::{Plugin, PluginManager};

struct MyPlugin {
    data: String,
}

impl Plugin for MyPlugin {
    fn name(&self) -> &str { "MyPlugin" }
    
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("Plugin initialized!");
        Ok(())
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Plugin update logic
        Ok(())
    }
}

// Add plugin to your app
app()
    .add_plugin(Box::new(MyPlugin { data: "Hello".to_string() }))
    .run(MyApp::new())
```

## Event System

The event system provides type-safe event handling:

```rust
use vetrace_engine::app::EventBus;

// Define custom events
#[derive(Clone)]
struct PlayerScored {
    player_id: u32,
    score: u32,
}

// In your app
let mut event_bus = EventBus::new();

// Subscribe to events
event_bus.subscribe::<PlayerScored>(|event| {
    println!("Player {} scored {}!", event.player_id, event.score);
});

// Emit events
event_bus.emit(PlayerScored { player_id: 1, score: 100 });
event_bus.process_events(); // Process all queued events
```

## Resource Management

Resources provide shared state across your application:

```rust
use vetrace_engine::app::ResourceManager;

#[derive(Clone)]
struct GameSettings {
    volume: f32,
    difficulty: u32,
}

// In your app
let mut resources = ResourceManager::new();
resources.insert(GameSettings { volume: 0.8, difficulty: 2 });

// Access resources
if let Some(settings) = resources.get::<GameSettings>() {
    println!("Volume: {}", settings.volume);
}
```

## Input Events

The framework provides unified input event handling:

```rust
use vetrace_engine::app::InputEvent;
use sdl2::keyboard::Keycode;

impl App for MyApp {
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {
        match event {
            InputEvent::KeyPressed { key } => {
                println!("Key pressed: {:?}", key);
            }
            InputEvent::KeyReleased { key } => {
                println!("Key released: {:?}", key);
            }
            InputEvent::MousePressed { button, x, y } => {
                println!("Mouse button {:?} pressed at ({}, {})", button, x, y);
            }
            InputEvent::MouseReleased { button, x, y } => {
                println!("Mouse button {:?} released at ({}, {})", button, x, y);
            }
            InputEvent::MouseMoved { x, y } => {
                // Handle mouse movement
            }
            InputEvent::WindowResized { width, height } => {
                println!("Window resized to {}x{}", width, height);
            }
        }
    }
}
```

## Editor Integration

For editor functionality, use the `vetrace_editor` crate:

```rust
// This will be available once the editor is separated
use vetrace_editor;

app()
    .add_plugin(vetrace_editor::editor())
    .run(MyApp::new())
```

## Migration from Legacy Engine

If you're migrating from the legacy engine approach:

1. **Replace `Engine::run_with_behaviour`** with the app framework
2. **Move UI code** to plugins or the editor crate
3. **Use the App trait** instead of implementing Behaviour directly
4. **Leverage the plugin system** for modular functionality

## Best Practices

1. **Keep apps lightweight**: Use plugins for complex functionality
2. **Use events for communication**: Avoid tight coupling between components
3. **Manage resources efficiently**: Clean up resources in the cleanup method
4. **Handle errors gracefully**: Use Result types in plugin methods
5. **Test incrementally**: Start with a simple app and add complexity gradually

## Examples

See the `examples/app_framework_demo.rs` file for a complete working example.
