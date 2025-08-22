# PBR Cat Example

A clean, modular example demonstrating PBR (Physically Based Rendering) with animated GLTF models in the Vetrace Engine.

## Features

- **GLTF Model Loading**: Loads and displays PBR materials from GLTF files
- **Animation System**: Supports translation, rotation, scale, and morph target animations
- **Animation Scaling**: Automatic and manual scaling of animation movements
- **Modular Architecture**: Clean separation of concerns across multiple modules

## Module Structure

### `main.rs`
The entry point that coordinates all other modules. Keeps the main function clean and focused.

### `asset_loader.rs`
Handles all asset loading operations:
- `load_cat_model()` - Loads the GLTF cat model
- `list_available_animations()` - Gets available animation names
- `animation_exists()` - Checks if a specific animation exists

### `animation_manager.rs`
Manages animation setup and debugging:
- `debug_animation_info()` - Prints detailed animation information
- `setup_animation()` - Configures animation with custom parameters
- `setup_first_available_animation()` - Auto-setup with sensible defaults

### `scene_setup.rs`
Handles scene configuration:
- `setup_camera()` - Creates and positions the camera
- `setup_lighting()` - Adds directional lighting
- `setup_complete_scene()` - Sets up both camera and lighting

## Animation Scaling

The example demonstrates the new animation scaling system that fixes the issue where animations were too dramatic when objects were scaled down:

### Automatic Scaling
The animation system automatically scales translation movements based on the object's size:
```rust
let object_scale = (transform.size[0] + transform.size[1] + transform.size[2]) / 3.0;
```

### Manual Scaling
You can also manually control animation intensity with the `translation_scale` field:
```rust
anim.translation_scale = 0.1; // 10% of original movement
```

### Scale Values
- `1.0` = Normal animation movement
- `0.1` = 10% of original (gentle movement)
- `0.5` = 50% of original
- `2.0` = 200% of original (more dramatic)

## Usage

```bash
cargo run --example pbr_cat
```

## Controls

- **Mouse**: Look around
- **WASD**: Move camera
- **Space/Shift**: Move up/down
- **Scroll**: Adjust movement speed

## Output

The example provides detailed logging:
```
Starting PBR Cat Example
Loading cat model...
Cat model loaded successfully with ID: 0
Available animations: ["Take 001"]
Animation 'Take 001' details:
  Duration: 5.97s
  Channels: 2
  Channel 0: Translation with 155 keyframes
  Channel 1: Rotation with 155 keyframes
Successfully configured animation: Take 001 (translation scale: 0.1)
Camera setup complete
Lighting setup complete
Scene setup complete. Starting engine...
```

## Extending the Example

The modular structure makes it easy to extend:

1. **Add new asset types** in `asset_loader.rs`
2. **Add animation controls** in `animation_manager.rs`
3. **Add post-processing effects** in `scene_setup.rs`
4. **Coordinate new features** in `main.rs`

This structure follows the single responsibility principle and makes the code much more maintainable and testable.
