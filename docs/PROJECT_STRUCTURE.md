# Project Structure

The repository is a Cargo workspace. Each major engine capability is isolated in its own crate.

```text
Vetrace/
├── vetrace_core/          Actor, ECS, queries, commands, events, scheduling
├── vetrace_render/        WGPU renderer and Actor render extensions
├── vetrace_physics/       Rapier integration and Actor physics extensions
├── vetrace_net/           Networking, RPC, and replication
├── vetrace_ui/            Runtime UI
├── vetrace_audio/         Audio integration
├── vetrace_animation/     Animation runtime
├── vetrace_scene/         Scene documents, SceneInstance, prefab APIs and compatibility
├── vetrace_primitives/    Actor-first primitive spawning
├── vetrace_editor/        Runtime editor
├── vetrace_map_builder/   Map-building application
├── vetrace_profiler/      Profiling and diagnostics
├── vetrace_scripting_lua/ Lua scripting integration
├── vetrace_engine/        Compatibility re-export crate
├── simple_shooter/        Multiplayer example game
├── scripts/               Validation scripts
└── third_party/           Vendored workspace dependencies
```

## Core layout

```text
vetrace_core/src/
├── actor/
│   ├── mod.rs
│   ├── handle.rs
│   ├── builder.rs
│   ├── error.rs
│   └── tests.rs
├── ecs/                   Generational Entity and World storage
├── engine/                Engine and component registry
├── query.rs               Typed immutable and scoped mutable queries
├── commands.rs            Deferred structural commands
├── bundle.rs              Bundle trait and tuple bundles
├── events.rs              Typed event channels
├── schedule.rs            Stages, named systems, fixed timestep
├── hierarchy.rs           Derived hierarchy index
├── systems/               Core timer and transform propagation
├── scene/                 Runtime-neutral scene definitions
└── app/                   App, Plugin, AppBuilder, application loop
```

## Dependency direction

- `vetrace_core` contains only generic engine foundations.
- Feature crates depend on core and provide subsystem components, bundles, plugins, and Actor extension traits.
- Games depend on the feature crates they use.
- Game-specific components and policy remain in the game crate.

## High-level module boundaries

- `vetrace_net/src/game_driver/`: compatibility, wire protocol/events, server driver, and client driver.
- `vetrace_scripting_lua/src/modding/`: manifests, sandbox API, runtime loading, and manager lifecycle.
- `vetrace_editor/src/windows/main_window/`: top panel, scene tree, inspector, and file explorer.
- `simple_shooter/src/app/gameplay/`: movement, shooting, damage, and respawn policy.
- `simple_shooter/src/app/player_visuals/`: player spawning, name labels, outlines, and shot trails.
- `simple_shooter/src/app/main_menu/`: page widgets/settings, server page, and action dispatch.

These directories use explicit Rust modules and narrow re-exports so call sites
consume a stable facade while implementation responsibilities stay isolated.

## Where new code belongs

- Generic lifecycle, query, scheduling, events, identity, or hierarchy: `vetrace_core`
- Rendering implementation or render Actor extensions: `vetrace_render`
- Rapier synchronization and physics Actor extensions: `vetrace_physics`
- General transport, RPC, or replication: `vetrace_net`
- Versioned scene/prefab persistence: `vetrace_scene`
- Shooter weapons, enemies, HUD, and rules: `simple_shooter`
