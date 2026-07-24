# Architecture

## Core boundary

`Engine` owns private ECS and resource storage. Normal game code cannot reach `engine.world`; it works through Actor, Query, Commands, Resources, Events, and Schedule.

`raw_world()` and `raw_world_mut()` remain explicit low-level escape hatches for integrations that genuinely need ECS-wide type-erased or backend operations, such as Rapier synchronization, render extraction, scene serialization, Lua bindings, and replication adapters.

## Runtime and persistent identity

`Entity` packs a slot index and generation into `u64`. Despawning or clearing increments the generation, invalidating stale handles.

`Actor` wraps Entity and adds the high-level invariant-preserving API.

`ActorId` is a UUID component created for every Actor. It is used for persistent references and scene identity. Runtime handles and persistent IDs are deliberately separate.

## Components and registry

Components remain ordinary Rust types. `ComponentManager` provides optional metadata:

- stable namespaced ID
- display name
- Rust `TypeId`
- serialization/deserialization callbacks
- clone/remove callbacks
- optional editor inspector callback

```rust
registry.register_serializable::<Health>(
    "my_game.health",
    "Health",
);
```

Registered serializable plugin components can round-trip through scene documents without `vetrace_scene` depending on the plugin crate.

## Queries

Immutable queries support typed tuples and filters. Mutable queries use callback scopes, preventing mutable component references from escaping while structural changes are deferred.

Queries return Actor handles, not raw entities.

## Commands

`Commands` queues structural operations:

- spawn
- insert/remove component
- recursive or single-Actor despawn
- custom deferred closures

The application loop flushes commands after each stage. This makes spawn/despawn/component removal safe while systems are iterating.

## Hierarchy and transforms

`Parent` is the one ECS source of truth. `Hierarchy` is rebuilt or incrementally maintained as a derived parent/child index.

`GlobalTransform` is derived from local `Transform` and Parent. Component change tracking and `TransformDirty` mark affected branches. Core scheduled systems propagate changes after normal updates and again before render extraction to capture late animation/plugin edits.

## Scheduling

The ordered stages are:

```text
Startup
PreUpdate
Update
FixedUpdate
Physics
PostPhysics
PostUpdate
RenderExtract
Render
Cleanup
```

`FixedTime` accumulates frame time and runs a bounded number of fixed simulation steps. Named systems can be inserted normally, before another named system, or after it.

```rust
AppBuilder::new()
    .add_system(Stage::Update, "game.input", input_system)
    .add_system_after(Stage::Update, "game.input", "game.movement", movement_system);
```

Core timers run at `FixedUpdate`. Core transform propagation is installed by `Engine::new`; apps do not need an optional hierarchy plugin.

## Events

`Events<T>` is a typed channel stored as a resource:

```rust
engine.send_event(PlayerDamaged { player, amount: 10 });
for event in engine.event_reader::<PlayerDamaged>() { /* ... */ }
let consumed = engine.drain_events::<PlayerDamaged>();
```

Actor destruction emits `ActorDestroyed` automatically.

## Scenes and prefabs

`SceneDocument::instantiate` returns `SceneInstance`, containing:

- root Actors
- every spawned Actor
- authoring scene-ID lookup
- ActorId lookup

Dropping a value does not unload it automatically. Call `instance.unload(engine)` to recursively destroy its roots.

Fluent prefab spawning:

```rust
let instance = engine
    .instantiate_prefab(&enemy_prefab)
    .named("Elite Enemy")
    .at(position)
    .child_of(room)
    .build()?;
```

## Dependency direction

- `vetrace_core` owns generic ECS/application architecture.
- Renderer, physics, networking, UI, audio, animation, and scripting depend on core.
- Feature crates extend Actor through traits and provide their own bundles.
- Games own game-specific components, bundles, rules, and systems.

Core must never depend on a game or on render/physics-specific component types.
