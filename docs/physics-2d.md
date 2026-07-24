# Optional 2D physics

Vetrace's 2D physics and collision code is compiled only when the `physics_2d` Cargo feature is enabled. It is renderer-neutral and independent from the Rapier 3D bridge.

## Enable it

Using the engine facade:

```toml
vetrace_engine = {
    path = "../vetrace_engine",
    default-features = false,
    features = ["render_egui", "render_2d", "physics_2d"]
}
```

Install the plugin after the renderer plugins when assembling an app directly:

```rust
AppBuilder::new()
    .add_plugin(RenderPlugin::new())
    .add_plugin(Render2dPlugin::new())
    .add_plugin(Physics2dPlugin::new());
```

`Physics2dPlugin` registers these authored components:

- `RigidBody2D`: static, dynamic, or kinematic motion policy.
- `Collider2D`: circle/box geometry, sensor mode, layers, masks, friction, and restitution.
- `Velocity2D`: linear and angular velocity.

The authoritative pose remains the normal core `Transform`.

## Collision events

Drain typed events during gameplay update:

```rust
for collision in engine.drain_events::<CollisionStarted2D>() {
    println!("{:?} touched {:?}", collision.entity_a, collision.entity_b);
}
```

Available channels are `CollisionStarted2D`, `CollisionContact2D`, and `CollisionStopped2D`. Sensors generate events without physical displacement or impulses.

## Queries

The feature includes renderer-independent query helpers:

- `raycast_2d`
- `point_query_2d`
- `overlap_circle_2d`
- `overlap_box_2d`

`Physics2dQueryFilter` controls accepted layers, sensor inclusion, and one excluded entity.

## Compile it out

```bash
cargo build -p vetrace_studio --no-default-features
```

With `physics_2d` disabled, the 2D body/collider components, solver, broad phase, narrow phase, query code, events, editor collider picking, and plugin registration are not compiled.

To keep 2D rendering but exclude 2D physics:

```bash
cargo build -p vetrace_studio --no-default-features --features render_2d
```

## Example game

```bash
cargo run -p vetrace_engine \
  --example top_down_2d \
  --features render_egui,render_2d,physics_2d
```

The example starts round one with three zombies. Each cleared round adds two more. WASD or arrow keys move, the mouse aims, and left click creates a continuously checked moving bullet.
