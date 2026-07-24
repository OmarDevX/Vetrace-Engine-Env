# Optional 2D rendering

Vetrace's 2D renderer is compiled only when the `render_2d` Cargo feature is enabled. The feature owns the 2D components, extraction code, camera resource, WGPU/SDL render paths, scene texture restoration, editor picking, and Studio controls.

## Engine facade

```toml
vetrace_engine = {
    path = "../vetrace_engine",
    default-features = false,
    features = ["render", "render_wgpu", "primitives", "scene", "render_2d"]
}
```

When assembling plugins directly, install the base renderer before the optional 2D plugin:

```rust
AppBuilder::new()
    .add_plugin(RenderPlugin::new())
    .add_plugin(Render2dPlugin::new());
```

`Render2dPlugin` registers `Camera2D`, `CanvasItem2D`, and `Sprite2D`. The runtime crate installs it automatically when its own `render_2d` feature is enabled.

## Compile it out

The following build does not compile any 2D Rust modules or WGSL source:

```bash
cargo build -p vetrace_studio --no-default-features
```

The default Studio build includes 2D support. To build the generic player without it, disable default features and re-enable only the required runtime features:

```bash
cargo build -p vetrace_player --no-default-features --features window,audio_backend,gltf
```

A feature-disabled build has no `Sprite2D`, `CanvasItem2D`, `Camera2D`, `Render2dPlugin`, 2D extraction, 2D WGPU pipeline, SDL 2D path, or Studio 2D commands in its compiled crate graph.

## Studio workflow

With `render_2d` enabled, Studio exposes 2D/3D viewport buttons and an **Add → Sprite 2D** command. In the 2D viewport:

- Right-drag pans the orthographic camera.
- The mouse wheel zooms around the cursor.
- Clicking selects the topmost sprite by canvas layer and z-index.
- Translate, rotate, and scale tools edit the normal Vetrace `Transform`.
- Texture assets can be dragged from Assets into the viewport to create a sprite at the drop position.

Texture paths remain project-relative in the scene. Runtime `TextureHandle` values are reconstructed when the scene is loaded.

## Optional 2D physics

The separate `physics_2d` feature adds `RigidBody2D`, `Collider2D`, `Velocity2D`, collision events, overlap/raycast queries, and Studio collider picking. It is not required for sprite rendering. See `docs/physics-2d.md`.
