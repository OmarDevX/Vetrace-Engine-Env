# Migration Guide: Engine Refactoring

## Overview

The Vetrace Engine has been refactored to separate the core engine from editor functionality. This provides better modularity and allows users to create applications without editor dependencies.

## Key Changes

### 🔄 **Editor UI Moved to Separate Crate**
- All editor UI (`MainWindow`, `SandboxWindow`, inspector, gizmos) moved to `vetrace_editor` crate
- Core engine no longer has editor dependencies
- Editor is now an optional plugin

### 🎯 **New Application Framework**
- New `App` trait for creating applications
- Plugin system for modular functionality
- Event system for plugin communication

## Migration Steps

### 1. Update Your Code Structure

**Before (Old Engine):**
```rust
use vetrace_engine::Engine;

fn main() {
    let mut engine = Engine::new(false);
    // Manual game loop
    while engine.running {
        engine.update();
        engine.render();
    }
}
```

**After (New Framework):**
```rust
use vetrace_engine::app::{app, App};
use vetrace_engine::engine::engine::Engine;

struct MyApp;

impl App for MyApp {
    fn setup(&mut self, engine: &mut Engine) {
        // Initialize your game
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        // Update game logic
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("My Game")
        .run(MyApp)
}
```

### 2. Add Editor if Needed

**If you want editor functionality:**

Add to `Cargo.toml`:
```toml
[dependencies]
vetrace_editor = { path = "../vetrace_editor" }
```

Update your code:
```rust
use vetrace_editor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("My Game with Editor")
        .add_plugin(vetrace_editor::editor())
        .run(MyApp)
}
```

### 3. Update UI References

**Before:**
```rust
use vetrace_engine::ui::{MainWindow, SandboxWindow};

// Direct access to editor windows
engine.main_window.selected_entities.clear();
engine.sandbox_window.skycolor = [1.0, 0.0, 0.0];
```

**After:**
```rust
// Editor UI is now handled by the plugin
// Use events or plugin interfaces for communication

// For game UI, use the new game UI system:
use vetrace_engine::ui::GameUIRenderer;
```

### 4. Update Component Registration

**Before:**
```rust
// Component registration was mixed with editor
engine.auto_register_component::<MyComponent>("MyComponent");
```

**After:**
```rust
// Same API, but now separated from editor
engine.auto_register_component::<MyComponent>("MyComponent");

// Editor plugin will automatically pick up registered components
```

### 5. Handle Events

**Before:**
```rust
// Direct manipulation of engine state
if some_condition {
    engine.main_window.selected_entities.push(entity);
}
```

**After:**
```rust
// Use event system for communication
use vetrace_engine::app::events::*;

// Send events that plugins can handle
engine.event_bus.send(EntitySelectedEvent { entity });
```

## Breaking Changes

### Removed from Engine Struct
- `engine.main_window` - moved to editor plugin
- `engine.sandbox_window` - moved to editor plugin
- Direct editor UI access

### Changed APIs
- `Engine::new()` - now part of app framework
- Manual game loop - replaced with App trait
- Direct UI manipulation - replaced with events

### New Dependencies
- Add `vetrace_editor` crate if you need editor functionality
- Update imports for UI components

## Benefits of Migration

### ✅ **Cleaner Separation**
- Core engine has no editor dependencies
- Smaller binary size for production builds
- Better modularity

### ✅ **Plugin Architecture**
- Easy to add/remove features
- Better extensibility
- Cleaner code organization

### ✅ **Modern API**
- App trait similar to Bevy/Godot
- Event-driven architecture
- Better error handling

## Common Migration Issues

### Issue: Missing MainWindow/SandboxWindow
**Problem:** Code references `engine.main_window` or `engine.sandbox_window`

**Solution:** 
1. Add `vetrace_editor` plugin if you need editor functionality
2. Use events for communication instead of direct access
3. Move editor-specific logic to custom plugins

### Issue: UI Components Not Found
**Problem:** `use vetrace_engine::ui::{MainWindow, SandboxWindow}` fails

**Solution:**
```rust
// Replace with:
use vetrace_editor::{MainWindow, SandboxWindow}; // If using editor plugin
// OR
use vetrace_engine::ui::GameUIRenderer; // For game UI
```

### Issue: Manual Game Loop
**Problem:** Old manual game loop doesn't work

**Solution:** Implement the `App` trait and use the app framework:
```rust
impl App for MyApp {
    fn setup(&mut self, engine: &mut Engine) { /* setup */ }
    fn update(&mut self, engine: &mut Engine, delta_time: f32) { /* update */ }
}
```

## Example Migration

### Before (Legacy):
```rust
use vetrace_engine::Engine;
use vetrace_engine::ui::{MainWindow, SandboxWindow};

fn main() {
    let mut engine = Engine::new(false);
    
    // Setup
    engine.sandbox_window.skycolor = [0.5, 0.8, 1.0];
    
    // Game loop
    while engine.running {
        // Update
        engine.update();
        
        // Custom logic
        if engine.main_window.selected_entities.len() > 0 {
            println!("Selected entities: {}", engine.main_window.selected_entities.len());
        }
        
        // Render
        engine.render();
    }
}
```

### After (New Framework):
```rust
use vetrace_engine::app::{app, App, InputEvent};
use vetrace_engine::engine::engine::Engine;
use vetrace_editor; // Only if you need editor

struct MyApp {
    selected_count: usize,
}

impl App for MyApp {
    fn setup(&mut self, engine: &mut Engine) {
        // Setup logic here
        println!("App initialized!");
    }
    
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        // Update logic here
        if self.selected_count > 0 {
            println!("Selected entities: {}", self.selected_count);
        }
    }
    
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {
        // Handle input events
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app()
        .with_title("My Game")
        .with_size(1280, 720)
        .add_plugin(vetrace_editor::editor()) // Optional: add editor
        .run(MyApp { selected_count: 0 })
}
```

## Getting Help

- Check the `APP_FRAMEWORK_GUIDE.md` for detailed framework documentation
- Look at examples in `examples/` directory
- See `vetrace_editor/` crate for editor plugin usage

The new architecture provides a much cleaner and more maintainable codebase while preserving all the original functionality!
