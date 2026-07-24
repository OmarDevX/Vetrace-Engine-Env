# Actor API

`Actor` is the normal game-facing handle for an ECS object. It serves the same role as a Unity `GameObject`, but components remain owned exclusively by the ECS.

## Identity

There are three related types:

- `Actor`: copyable, generational runtime handle.
- `Entity`: low-level packed runtime handle used by subsystem internals.
- `ActorId`: UUID-backed persistent identity for scenes, saves, editor references, and networking.

A destroyed Actor cannot become valid again when its ECS slot is reused because the generation changes.

```rust
let id = actor.id(&engine).unwrap();
let same_actor = engine.find_actor_by_id(id);
```

Do not persist `Actor` as object identity. Persist `ActorId`.

## Spawn an Actor

```rust
let enemy = engine
    .spawn_actor("Enemy")
    .with(Transform::default())
    .with(Health(100))
    .tag("enemy")
    .source("my_game")
    .build();
```

Every Actor starts with:

- `ActorId`
- `Name`
- `Transform`
- `GlobalTransform`
- `Metadata`

Use `try_build()` when components or hierarchy relationships come from data:

```rust
let weapon = engine
    .spawn_actor("Weapon")
    .child_of(enemy)?
    .try_build()?;
```

An unfinished or failed builder rolls back its partially created Actor.

## Bundles

Bundles keep common spawn definitions reusable:

```rust
struct EnemyBundle {
    transform: Transform,
    health: Health,
}

impl Bundle for EnemyBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), ActorError> {
        actor.insert(engine, self.transform)?;
        actor.insert(engine, self.health)?;
        Ok(())
    }
}

let enemy = engine
    .spawn_actor("Enemy")
    .bundle(EnemyBundle {
        transform: Transform::default(),
        health: Health(100),
    })
    .build();
```

Tuples of up to eight components also implement `Bundle`.

## Components

```rust
if let Some(health) = enemy.get_component::<Health>(&engine) {
    println!("health: {}", health.0);
}

if let Some(health) = enemy.get_component_mut::<Health>(&mut engine) {
    health.0 -= 25;
}

enemy.insert(&mut engine, Health(150))?;
let previous = enemy.remove::<Health>(&mut engine);
```

Core invariant components have dedicated APIs. Direct mutation of `ActorId`, `Parent`, `GlobalTransform`, hierarchy compatibility data, and dirty markers is blocked.

## Transforms

```rust
enemy.set_position(&mut engine, Vec3::new(4.0, 0.0, 2.0))?;
enemy.translate(&mut engine, Vec3::Y)?;
enemy.set_rotation(&mut engine, Quat::IDENTITY)?;
enemy.set_scale(&mut engine, Vec3::splat(2.0))?;
```

Transform changes are tracked. The engine refreshes `GlobalTransform` automatically during the standard schedule. Tools that need world-space values immediately after a batch edit can call:

```rust
engine.sync_transforms();
```

## Hierarchy

`Parent` is the authoritative relationship. `Hierarchy` is a derived traversal index.

```rust
weapon.set_parent(&mut engine, enemy)?;
enemy.add_child(&mut engine, weapon)?;
weapon.clear_parent(&mut engine)?;

let parent = weapon.parent(&engine);
let children = enemy.children(&engine);
```

The API rejects dead parents, self-parenting, and cycles.

## Destruction

```rust
enemy.despawn(&mut engine);       // Actor and all descendants
weapon.despawn_only(&mut engine); // only this Actor; children become roots
```

Destruction emits `ActorDestroyed` through the typed event system.

## Queries

Immutable tuple queries support one to four components plus filters:

```rust
for (actor, health, transform) in engine
    .query::<(&Health, &Transform)>()
    .with::<Enemy>()
    .without::<Disabled>()
{
    println!("{:?}: {}", transform.translation, health.0);
}
```

Mutable queries use callbacks so references cannot escape:

```rust
engine
    .query_mut_with::<Health, DamagePerSecond>()
    .without::<Dead>()
    .for_each(|actor, health, damage| {
        health.0 -= damage.0;
    });
```

Structural changes can be queued directly from a mutable query:

```rust
engine.query_mut::<Health>().for_each_with_commands(
    |actor, health, commands| {
        if health.0 <= 0 {
            commands.despawn(actor);
        }
    },
);
```

Commands are applied at the normal stage boundary or by `engine.flush_commands()`.

## Subsystem extensions

Feature crates extend Actor without making core depend on them:

```rust
use vetrace_render::RenderActorExt;
use vetrace_physics::PhysicsActorExt;

actor.set_visible(&mut engine, true)?;
actor.set_velocity(&mut engine, Vec3::new(0.0, 5.0, 0.0))?;
```

## Access boundary

Preferred gameplay code:

```rust
actor.get_component_mut::<Health>(&mut engine)
engine.query::<(&Health, &Transform)>()
```

Low-level subsystem code only:

```rust
engine.raw_world()
engine.raw_world_mut()
```
