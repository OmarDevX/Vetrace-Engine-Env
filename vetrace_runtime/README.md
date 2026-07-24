# vetrace_runtime

`vetrace_runtime` is the project-driven product boundary above Vetrace's low-level engine crates. It owns runtime lifecycle while keeping editor code and game-specific Rust code out of the player.

## Responsibilities

- Validate a `VetraceProject` before startup.
- Install only the project-enabled engine plugins.
- Select headless, WGPU, or SDL rendering backends.
- Map project window, rendering, physics, input, and feature settings into engine resources.
- Load explicit Lua autoload scripts.
- Load and instantiate the configured main scene.
- Resolve scene-attached Lua scripts through safe project-relative paths.
- Provide explicit start, update, pause, resume, reload, and stop control.
- Keep an inspectable runtime status, active-scene record, capabilities, and diagnostics resource.

## Cargo features

- `window`: WGPU + winit window backend.
- `software_window`: SDL software window backend.
- `audio_backend`: Kira audio backend.
- `gltf`: GLTF animation loading and GLTF-generated physics colliders.

The crate has no default native-window/audio features so headless tests remain portable. The future `vetrace_player` binary should enable the production feature set.

## Minimal embedding

```rust
use vetrace_project::VetraceProject;
use vetrace_runtime::{RuntimeMode, VetraceRuntime};

let project = VetraceProject::load("examples/lua_runtime_project")?;
let mut runtime = VetraceRuntime::load(project, RuntimeMode::StandaloneGame)?;
runtime.run_until_stopped(None, 1.0 / 60.0)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification

```bash
cargo test -p vetrace_core
cargo test -p vetrace_scripting_lua
cargo test -p vetrace_runtime
cargo check -p vetrace_runtime --features window,audio_backend,gltf
cargo check --workspace
```
