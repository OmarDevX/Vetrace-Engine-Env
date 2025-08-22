# Vetrace Engine - App Framework & Plugin System Guide

## Overview

The Vetrace Engine now features a modern **Application Framework** with a **Plugin System** that provides clean separation between the core engine and optional features like the editor. This architecture is inspired by successful engines like Bevy and Godot.

## Key Benefits

- 🎯 **Clean Separation**: Core engine is independent of editor functionality
- 🔌 **Plugin Architecture**: Features can be added/removed as needed
- 🚀 **Easy to Use**: Simple App trait for creating applications
- 🔄 **Event System**: Plugins can communicate through typed events
- 📦 **Modular**: Editor is now a separate crate (`vetrace_editor`)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Application                         │
│                   (implements App)                          │
├─────────────────────────────────────────────────────────────┤
│                   App Framework                             │
│              (Plugin Manager + Event Bus)                   │
├─────────────────────────────────────────────────────────────┤
│  Plugin 1        Plugin 2        Plugin 3                  │
│ (Editor)       (Audio)         (Networking)                │
├─────────────────────────────────────────────────────────────┤
│                  Vetrace Engine Core                        │
│           (ECS, Rendering, Assets, Input)                   │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Basic Application

```rust
use vetrace_engine::app::{app, App};
use vetrace_engine::engine::engine::Engine;

struct MyApp;

impl App for MyApp {
    fn setup(&mut self, engine: &mut Engine) {
        println!("App starting!");
        // Initialize your game/application
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        // Update game logic
    }
    
    fn render(&mut self, engine: &mut Engine) {
        // Custom rendering (optional)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("My Game")
        .with_size(1280, 720)
        .run(MyApp)
}
```

### 2. Application with Editor

```rust
use vetrace_engine::app::{app, App};
use vetrace_editor; // Separate crate

struct MyGameWithEditor;

impl App for MyGameWithEditor {
    fn setup(&mut self, engine: &mut Engine) {
        // Create your game scene
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("My Game with Editor")
        .add_plugin(vetrace_editor::editor()) // Add editor plugin
        .run(MyGameWithEditor)
}
```

## Plugin System

### Creating a Plugin

```rust
use vetrace_engine::app::plugin::Plugin;
use vetrace_engine::engine::engine::Engine;

struct MyPlugin {
    initialized: bool,
}

impl Plugin for MyPlugin {
    fn name(&self) -> &'static str {
        "my_plugin"
    }
    
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing MyPlugin");
        self.initialized = true;
        Ok(())
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Plugin update logic
        Ok(())
    }
    
    fn dependencies(&self) -> Vec<&'static str> {
        vec!["core_systems"] // Plugins this depends on
    }
}
```

### Using Plugins

```rust
app()
    .add_plugin(MyPlugin::new())
    .add_plugin(AnotherPlugin::new())
    .run(MyApp::new())
```

## Event System

### Defining Events

```rust
use vetrace_engine::app::events::Event;

#[derive(Debug, Clone)]
struct PlayerDiedEvent {
    player_id: u32,
    cause: String,
}

impl Event for PlayerDiedEvent {}
```

### Sending Events

```rust
// In your app or plugin
engine.event_bus.send(PlayerDiedEvent {
    player_id: 1,
    cause: "Fell into lava".to_string(),
});
```

### Handling Events

```rust
// Subscribe to events
engine.event_bus.subscribe_fn(|event: &PlayerDiedEvent| {
    println!("Player {} died: {}", event.player_id, event.cause);
});

// Process events each frame
engine.event_bus.process_events();
```

## Editor Plugin

The editor is now completely separate from the core engine:

### Features
- 🎨 **Scene Editor**: Visual scene editing with gizmos
- 🔍 **Inspector**: Component property editing
- 📁 **File Explorer**: Asset management
- 🎮 **Sandbox**: Object creation tools
- 🎯 **Selection**: Entity picking and multi-selection

### Usage

Add to your `Cargo.toml`:
```toml
[dependencies]
vetrace_editor = { path = "../vetrace_editor" }
```

Use in your application:
```rust
use vetrace_editor;

app()
    .add_plugin(vetrace_editor::editor())
    .run(MyApp::new())
```

## App Framework API

### App Trait Methods

```rust
trait App {
    fn setup(&mut self, engine: &mut Engine) {}
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}
    fn render(&mut self, engine: &mut Engine) {}
    fn cleanup(&mut self, engine: &mut Engine) {}
    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {}
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {}
}
```

### AppBuilder Methods

```rust
app()
    .with_title("My Game")           // Set window title
    .with_size(1280, 720)           // Set window size
    .with_vsync(true)               // Enable/disable vsync
    .add_plugin(MyPlugin::new())    // Add plugins
    .add_resource(MyResource::new()) // Add shared resources
    .with_event_bus(|bus| {         // Configure event bus
        bus.set_immediate_mode(true);
    })
    .run(MyApp::new())              // Run the application
```

## Examples

### 1. Basic Game Loop
See `examples/app_framework_demo.rs` for a simple application without editor.

### 2. Game with Editor
See `examples/editor_demo.rs` for an application with the full editor interface.

### 3. Custom Plugin
See the examples for how to create and use custom plugins.

## Migration Guide

### From Legacy Engine

**Old way:**
```rust
let mut engine = Engine::new()?;
// Manual setup and game loop
```

**New way:**
```rust
app().run(MyApp::new())?;
```

### Benefits of Migration

1. **Cleaner Code**: No manual game loop management
2. **Plugin Support**: Easy to add/remove features
3. **Better Testing**: Easier to test individual components
4. **Separation of Concerns**: Editor doesn't bloat your game
5. **Future-Proof**: Extensible architecture

## Best Practices

1. **Keep Apps Simple**: Put complex logic in plugins or systems
2. **Use Events**: Communicate between plugins through events
3. **Plugin Dependencies**: Declare dependencies explicitly
4. **Resource Management**: Use the resource system for shared data
5. **Error Handling**: Always handle plugin errors gracefully

## Workspace Structure

```
├── vetrace_engine/          # Core engine
├── vetrace_editor/          # Editor plugin (separate crate)
├── Cargo.toml              # Workspace configuration
└── examples/               # Example applications
```

## Next Steps

1. **Try the Examples**: Run the demo applications
2. **Create Your App**: Implement the App trait for your game
3. **Add Plugins**: Use the editor or create custom plugins
4. **Extend the System**: Add your own events and resources

The new architecture provides a solid foundation for building games and tools with Vetrace Engine while maintaining clean separation between core functionality and optional features.
