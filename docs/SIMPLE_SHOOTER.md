# Simple Shooter

`simple_shooter` is the example game and the reference for Vetrace's game-facing APIs.

## Run and multiplayer

Open the game:

```bash
cargo run -p simple_shooter
```

The main menu owns the normal multiplayer flow:

- Select **Servers**.
- Select **Host** to start an authoritative server on this machine.
- Other players on the same LAN select **Servers**, choose the advertised
  server, and select **Join**.
- The host opens or closes the configuration panel with **F1**. The host can
  select a map, enable or disable discovered Lua mods, toggle bots,
  tune player speed, gravity, and jump strength, and start the match.
- Multiplayer waits in a dedicated compact lobby map. Starting replaces the
  lobby geometry and collision with the selected game map on every peer.
- Lobby combat remains active. Kills and deaths are authoritative and appear in
  the shared kills leaderboard.

LAN discovery uses UDP port `34557`; gameplay defaults to UDP port `3456`.
Firewalls must allow both ports. Internet hosting still requires the router to
forward the gameplay UDP port; automatic public directory/NAT traversal is not
part of this local server browser.

## Optional features

- `gltf`: glTF import and animation
- `editor`: runtime editor integration
- `profiler`: profiler UI and timing output
- `audio`: audio backend; enabled by default

```bash
cargo run -p simple_shooter --features gltf,editor,profiler -- --editor --profile
```

Graphics, VSync, volumetric fog, vignette, mouse sensitivity, and volume can
be changed from the main or pause-menu Settings page.

## Actor-first gameplay

Player spawning returns `Actor` and uses a game-owned bundle:

```rust
let player = engine
    .spawn_actor(display_name)
    .bundle(ShooterPlayerBundle { /* components */ })
    .tag("player")
    .source("simple_shooter")
    .build();
```

Per-player functions accept Actor instead of Entity:

```rust
fn damage_player(engine: &mut Engine, target: Actor, damage: i32) -> bool
fn teleport_player_body(engine: &mut Engine, actor: Actor, position: Vec3, velocity: Vec3)
```

Typed gameplay iteration uses `Query`:

```rust
for (actor, player, transform) in engine.query::<(&ShooterPlayer, &Transform)>() {
}
```

Networking uses `network_actor` and Actor-returning client mapping helpers. Raw Entity remains only at deliberate subsystem boundaries, such as raycast callbacks and generic replication adapter methods that operate on `World`.

## Engine boundary

Do not introduce normal gameplay calls to:

```rust
engine.raw_world()
engine.raw_world_mut()
```

Use:

```rust
actor.get_component::<T>(engine)
actor.get_component_mut::<T>(engine)
engine.query::<Q>()
engine.query_mut::<T>()
engine.defer(|commands| { /* structural changes */ })
```

## Weapon architecture

Simple Shooter keeps weapon policy entirely game-side:

- `WeaponRegistry` loads every `assets/weapons/*.json` definition by stable ID.
- `EquippedWeapon` stores per-player selection and cooldown state.
- input and bots emit `FireRequest`; authoritative simulation produces
  `ShotResult`; networking and presentation consume the result independently.
- `PlayerVisualOwner` tracks render-only outlines, labels, and weapon models.
- local players receive separate first-person and world weapon presentations.
- gameplay hashes cover IDs, damage, range, cooldown, and aim mode. Visual
  model/effect/audio replacement does not change multiplayer compatibility.

The runtime presentation pipeline is ordered as camera, owned-visual sync,
shot-result effects, UI, then post-physics transform synchronization. A build
without a presentation feature drains shot events without creating model,
tracer, flash, label, or outline actors, allowing authoritative headless use.

Do not move `WeaponDefinition`, `FireRequest`, hit rules, or weapon visuals into
engine crates. Reusable engine crates provide events, raycasts, rendering,
audio, hierarchy, and replication; the game composes them.
