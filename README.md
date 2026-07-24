# Vetrace

Vetrace is a modular game engine written in Rust. Its game-facing API is centered on `Actor`, a lightweight generational handle over the ECS.

The workspace includes rendering, physics, networking, UI, audio, scenes, scripting, editor tools, and a multiplayer example game.

## Quick start

Install a recent stable Rust toolchain, then run from the workspace root:

```bash
cargo run -p simple_shooter
```

Workspace validation:

```bash
cargo check --workspace
cargo test --workspace
```

## Website and browser examples

Vetrace includes a static engine website and a real WebAssembly runtime. Desktop windows and browser canvases now construct the same `vetrace_render::WgpuRenderer`; the browser does not maintain a separate reduced shader or mesh pipeline.

```bash
./scripts/build_web.sh
./scripts/serve_web.sh
```

Open `http://127.0.0.1:8080/website/`. See [Vetrace on the web](docs/WEB.md) for the current browser capability matrix and deployment notes.

Maintainability report (informational; it does not fail CI):

```bash
./scripts/module_size_report.sh       # default: modules over 400 lines
./scripts/module_size_report.sh 500   # custom threshold
```

The report considers Rust source modules only. It excludes tests, third-party
code, shaders, scene JSON, and generated build output, and also reports textual
`include!` usage that can hide module boundaries.

## Actor-first example

```rust
use vetrace_core::{Bundle, Engine, Transform};

#[derive(Clone, Copy)]
struct Health(i32);

let mut engine = Engine::new();
let player = engine
    .spawn_actor("Player")
    .bundle((Transform::default(), Health(100)))
    .tag("player")
    .build();

if let Some(health) = player.get_component_mut::<Health>(&mut engine) {
    health.0 -= 10;
}

for (actor, health, transform) in engine.query::<(&Health, &Transform)>() {
    println!("{}: {} at {:?}", actor.name(&engine).unwrap_or("Actor"), health.0, transform.translation);
}
```

Runtime `Actor`/`Entity` handles are generational. Persistent scene, save, editor, and network identity uses `ActorId`.

## Main architecture

- `Actor`: safe runtime object handle and normal gameplay API.
- `ActorId`: UUID-backed persistent identity.
- `Bundle`: reusable component sets.
- `Query`: typed one-to-four-component queries with `with`/`without` filters.
- `Commands`: deferred structural changes during systems and mutable queries.
- `Stage`/`Schedule`: named startup, fixed simulation, physics, update, render, and cleanup stages.
- `Events<T>`: typed event channels.
- `ComponentManager`: stable namespaced component registration for scenes and editor tooling.
- `SceneInstance`: actor-first scene/prefab loading and unloading.
- Render and physics extension traits add subsystem behavior without coupling `vetrace_core` to those crates.

## Documentation

- [Getting started](docs/GETTING_STARTED.md)
- [Actor API](docs/ACTOR.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Project structure](docs/PROJECT_STRUCTURE.md)
- [Simple Shooter](docs/SIMPLE_SHOOTER.md)
- [Runtime UI](docs/UI.md)
- [Lua mods](docs/LUA_MODS.md)
- [Build and export](docs/BUILD_EXPORT.md)
- [Web runtime and examples](docs/WEB.md)

## Main crates

| Crate | Purpose |
| --- | --- |
| `vetrace_core` | ECS, Engine, Actor, queries, commands, events, scheduling |
| `vetrace_project` | Versioned project manifests and safe project paths |
| `vetrace_asset` | Generic asset database, importers, cache, and file watching |
| `vetrace_build` | Toolchain-free `.vpak` packaging and player-template export |
| `vetrace_runtime` | Generic project runtime used by the player and Studio |
| `vetrace_player` | Generic executable for projects and packaged games |
| `vetrace_studio` | Project manager and editor application |
| `vetrace_render` | WGPU rendering and Actor render extensions |
| `vetrace_physics` | Rapier integration and Actor physics extensions |
| `vetrace_net` | Networking, RPCs, replication, and Actor-first helpers |
| `vetrace_ui` | Runtime UI |
| `vetrace_web` | Browser input/canvas adapter, normal AppRunner bridge, and live WebAssembly examples |
| `vetrace_audio` | Audio integration |
| `vetrace_scene` | Scene serialization, instances, prefab builders, and legacy prefab compatibility |
| `vetrace_editor` | Runtime editor tools |
| `vetrace_map_builder` | Map-building application |
| `simple_shooter` | Multiplayer example game |

## Boundary rule

Gameplay code should use `Actor`, typed queries, bundles, commands, resources, events, and subsystem extension traits. `Engine::raw_world()` is a documented low-level escape hatch for renderer, physics, editor, serialization, and replication internals—not a second gameplay API.
