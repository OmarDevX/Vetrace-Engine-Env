# Vetrace Engine Refactoring Guide

## Overview

The Vetrace Engine has been refactored from a monolithic structure into a clean, modular architecture. This guide explains the new structure and how to migrate existing code.

## The Problem

The original `Engine` struct had **42+ fields** and was responsible for everything:
- Rendering and graphics
- Input and window management  
- Scripting and behaviors
- Component management
- Event handling
- UI management
- Physics
- Asset management

This made the code:
- **Hard to understand** - too many responsibilities in one place
- **Difficult to maintain** - changes affected multiple unrelated systems
- **Hard to test** - tightly coupled components
- **Poor separation of concerns** - everything mixed together

## The Solution: Modular Architecture

The new architecture separates concerns into **specialized managers**:

### 1. **RenderingManager**
Handles all rendering-related functionality:
- Renderer backend (WGPU/OpenGL)
- Scene management
- EGUI integration

### 2. **InputManager**
Manages input and window operations:
- SDL context and window management
- Input handling and events
- Free flight camera controls

### 3. **ScriptingManager**
Handles scripting and behaviors:
- Lua script integration
- Behavior system
- Component behaviors
- Script lifecycle management

### 4. **ComponentManager**
Manages ECS component system:
- Component registration and factories
- Component editors and inspectors
- Generated component specs
- Component lifecycle functions

### 5. **EventManager**
Handles events and engine state:
- Collision events
- Entity events
- Scene events
- Engine state (running, paused, 2D mode)

### 6. **UIManager**
Manages UI and scene management:
- Sandbox window
- Main window
- Scene manager
- Scene serialization

## New Engine Structure

```rust
pub struct Engine {
    // Core engine systems
    pub core: EngineCore,
    pub world: World,
    pub physics: PhysicsState,
    pub assets: Arc<AssetManager>,

    // Specialized managers
    pub rendering: RenderingManager,
    pub input: InputManager,
    pub scripting: ScriptingManager,
    pub components: ComponentManager,
    pub events: EventManager,
    pub ui: UIManager,
}
```

## Benefits

### ✅ **Clear Separation of Concerns**
Each manager has a single, well-defined responsibility.

### ✅ **Improved Maintainability**
Changes to rendering don't affect input handling, etc.

### ✅ **Better Testability**
Each manager can be tested independently.

### ✅ **Easier to Understand**
Developers can focus on one aspect at a time.

### ✅ **Modular Development**
Teams can work on different managers simultaneously.

### ✅ **Future-Proof**
Easy to add new managers or replace existing ones.

## Migration Guide

### For Engine Users

The public API remains largely the same. Most existing code will work without changes:

```rust
// Old way (still works)
let mut engine = Engine::new(false);
let entity = engine.spawn_empty("test");
engine.world.insert(entity, Transform::default());

// New way (optional)
let mut engine = EngineBuilder::new()
    .with_2d_mode(false)
    .build()?;
```

### For Engine Developers

When working on engine internals, use the appropriate manager:

```rust
// Rendering operations
engine.rendering.renderer_mut().do_something();
engine.rendering.scene_mut().add_object(obj);

// Input operations  
engine.input.input_mut().handle_event(event);
engine.input.window_mut().set_title("New Title");

// Component operations
engine.components.register_factory("MyComponent", factory);
engine.components.add_generated_component("MyComponent".to_string());

// Event operations
engine.events.add_collision_event(collision);
engine.events.set_paused(true);
```

## Convenience Methods

The main `Engine` struct provides convenience methods for common operations:

```rust
// Direct access to commonly used systems
engine.renderer()          // Get renderer reference
engine.scene()             // Get scene reference  
engine.input()             // Get input reference
engine.spawn_empty("name") // Spawn entity
engine.is_running()        // Check engine state
```

## Builder Pattern

The new `EngineBuilder` provides a fluent API for engine creation:

```rust
let engine = EngineBuilder::new()
    .with_2d_mode(true)
    .build()?;

// Or use convenience functions
let engine = create_engine()?;        // 3D engine
let engine = create_2d_engine()?;     // 2D engine
```

## Legacy Compatibility

The original `Engine` struct is still available as `LegacyEngine` for gradual migration:

```rust
use vetrace_engine::engine::LegacyEngine;
```

## Next Steps

This refactoring provides the foundation for further improvements:

1. **System Manager** - Organize ECS systems into logical groups
2. **Rendering Pipeline** - Separate render passes and material management
3. **Plugin System** - Allow external modules to extend functionality
4. **Performance Optimization** - Optimize individual managers independently

## Example: Refactored PBR Cat

The `pbr_cat` example demonstrates the new modular approach:

```rust
// Clean, focused modules
mod asset_loader;      // Handles GLTF loading
mod animation_manager; // Manages animations  
mod scene_setup;       // Sets up camera/lighting

// Simple, coordinated main function
fn main() {
    let mut engine = Engine::new(false);
    
    let cat_id = AssetLoader::load_cat_model(&mut engine)?;
    AnimationManager::setup_first_available_animation(&mut engine, cat_id)?;
    SceneSetup::setup_complete_scene(&mut engine);
    
    engine.run(true);
}
```

This refactoring makes the Vetrace Engine much more maintainable, understandable, and extensible!
