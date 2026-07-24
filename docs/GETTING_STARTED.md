# Getting Started

## Requirements

- A recent stable Rust toolchain
- A graphics driver supported by WGPU for windowed rendering

## Run the example game

From the repository root:

```bash
cargo run -p simple_shooter
```

The default features include the window and audio backend.

## Build and test

```bash
cargo check --workspace
cargo test --workspace
cargo build --workspace --release
```

Optional features can be enabled when needed:

```bash
cargo run -p simple_shooter --features gltf,editor,profiler -- --editor --profile
```

For multiplayer, open **Servers** from the main menu. Host a LAN game or select
an advertised server to join it.

## Common controls

- `WASD`: move
- Mouse: look
- Left mouse button: shoot
- `F1`: open or close host lobby controls
- `Space`: jump
- `F10`: toggle editor mode when built with the `editor` feature
- `Escape`: open or close the pause menu; deselect in editor mode

Run the built-in help for all command-line options:

```bash
cargo run -p simple_shooter -- --help
```

## Next steps

- Read [Actor API](ACTOR.md) for spawning and component access.
- Read [Architecture](ARCHITECTURE.md) for queries, commands, events, stages, scenes, and identity.
- Read [Simple Shooter](SIMPLE_SHOOTER.md) for multiplayer and optional features.
