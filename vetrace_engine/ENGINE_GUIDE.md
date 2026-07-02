# Vetrace Engine Guide

## Table of Contents
- [Architecture Overview](#architecture-overview)
- [Entity Component System](#entity-component-system)
- [Custom Components](#custom-components)
- [Rust Behaviours](#rust-behaviours)
- [Lua Scripting](#lua-scripting)
- [Scene Files](#scene-files)
- [Input Handling](#input-handling)
- [Event System](#event-system)
- [UI System](#ui-system)
- [Rendering Pipeline](#rendering-pipeline)
- [Spawning Objects](#spawning-objects)
- [Engine Loop](#engine-loop)
- [Extending the Engine](#extending-the-engine)
- [Project Structure](#project-structure)
- [Player Example](#player-example)
- [LookAt Component](#lookat-component)
- [Collision System](#collision-system)
- [Sprite3D Component](#sprite3d-component)
- [Particle System](#particle-system)
- [Networking](#networking)

## Architecture Overview
Vetrace Engine is a lightweight wgpu-based game engine written in Rust. It
uses a simple ECS, supports behaviours written in Rust and Lua, loads scenes
from JSON and provides an egui based editor.

The entry point for library users is `Engine` in `src/engine/engine.rs`.

## Entity Component System
Entities are represented by the `Entity` struct which is just a wrapper around an
`u32` identifier. Components implement the `Component` trait. The `World` type
stores component maps and provides query helpers.

```rust
pub trait Component: 'static + Send + Sync {}
```

```rust
pub struct Entity(pub u32);
```

```rust
pub struct World {
    next_id: u32,
    components: HashMap<TypeId, Box<dyn Any>>,
    entities: Vec<Entity>,
}
```

Components are inserted with `world.insert(entity, component)` and accessed via
query methods such as `query2_mut`.

## Custom Components
Use `Engine::register_component` or `Engine::auto_register_component` to expose a
component to the editor and scene system. Registration also wires up add/remove
functions and UI editors.

```rust
pub fn register_component<T: Component + Default + 'static>(
    &mut self,
    name: &str,
    editor: fn(&mut T, &mut egui::Ui),
)
```
【F:src/engine/engine.rs†L308-L339】

`auto_register_component` automatically uses `Inspectable::draw_ui` for the UI.

```rust
pub fn auto_register_component<T: Component + Default + Inspectable + 'static>(
    &mut self,
    name: &str,
)
```
【F:src/engine/engine.rs†L894-L905】

## Rust Behaviours
Behaviours implement the `Behaviour` trait. They are updated every frame by the
engine.

```rust
pub trait Behaviour {
    fn start(&mut self, engine: &mut Engine) {}
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}
}
```
【F:src/ecs/behaviour.rs†L1-L7】

A simple behaviour rotating all entities with `Transform` and `Rotate` components
is implemented in `Rotator`:

```rust
impl Behaviour for Rotator {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        for (_e, t, r) in engine.world.query2_mut::<Transform, Rotate>() {
            let q = Quaternion::new(t.orientation[3], t.orientation[0], t.orientation[1], t.orientation[2]);
            let rot_y = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), r.speed * delta);
            let new_q = rot_y * UnitQuaternion::from_quaternion(q);
            let q_new = new_q.quaternion();
            t.orientation[0] = q_new.i;
            t.orientation[1] = q_new.j;
            t.orientation[2] = q_new.k;
            t.orientation[3] = q_new.w;
        }
    }
}
```
【F:src/behaviour/rotator.rs†L1-L43】

Add behaviours with `engine.add_behaviour(MyBehaviour)` before running.

## Lua Scripting
`ScriptComponent` stores the name of a Lua file. When present, the engine loads
the script into a `ScriptBehaviour` which calls `start(engine, self)` and
`update(engine, self, input, delta)` functions defined in the Lua file.

```rust
pub fn start(&self, engine: &mut Engine, entity: u32) { /* call Lua start */ }
pub fn update(&self, engine: &mut Engine, entity: u32, input: InputProxy, delta_time: f32) {
    /* call Lua update */
}
```
【F:src/behaviour/script.rs†L140-L170】

Lua scripts can access components through a proxy object provided to them.

Example Lua script:
```lua
function start(engine, self)
    -- called once when the entity is created
end

function update(engine, self, input, dt)
    local transform = self.Transform
    if transform then
        transform.position_x = transform.position_x + dt * 1.0
    end
end
```
Place scripts under `generated/behaviours` and call `engine.reload_scripts()` to
hot reload.

## Scene Files
Scenes are stored as JSON. `SceneFile` contains a list of `NodeFile` entries.

```rust
pub struct NodeFile {
    pub name: String,
    pub tags: Vec<String>,
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub size: [f32; 3],
    pub scale: [f32; 3],
    pub is_cube: bool,
    pub components: Vec<ComponentFile>,
}
```
【F:src/scene/loader.rs†L16-L28】

Load a scene programmatically with
`engine.load_scene_from_file("path.json")`. Each node spawns an object and its
components are created using the registered factories. You can also export the
current state of the world by calling `engine.save_scene_to_file("out.json")`.
Both helper functions operate independently of the editor UI, making it easy to
load or save scenes from your own code.

## Input Handling
`Input` collects SDL2 events each frame. Behaviour code can query keyboard or
mouse state:

```rust
if engine.input.is_key_down(Keycode::W) { /* ... */ }
let (x, y) = engine.input.mouse_position();
```
【F:src/input/mod.rs†L61-L94】

Lua scripts receive an `EngineHandle` allowing them to inspect input via exposed
API if desired. When updating UI elements from a script, call
`engine:request_redraw()` so the changes appear immediately.
Use `engine:print("msg")` to log from Lua when debugging.
You can also subscribe to input events from Lua:

```lua
function start(engine, self)
    engine.input:subscribe_key_down(function(key)
        print('pressed', key)
    end)
end
```

## Lua Networking
`EngineHandle` can create simple UDP clients and servers. Use `engine:create_client(addr)`
to connect to a server or `engine:create_server(addr)` to listen for clients.
Both return userdata objects with `poll`, `send_custom` and `recv` helpers for
sending and receiving custom packets.

```lua
function start(engine, self)
    server = engine:create_server('127.0.0.1:4000')
end

function update(engine, self)
    server:poll()
    local msg = server:recv()
    if msg then
        engine:print(msg.kind .. ' ' .. msg.data)
    end
end
```

## Event System
The engine exposes a small event type that Lua scripts can use to communicate
between entities. Create an event once and share it among multiple objects:

```lua
-- MyScript.lua shared by two entities
local ping_event

function start(engine, self)
    if not ping_event then
        -- first time this script runs create the event
        ping_event = engine:create_event()
    end

    if self.name == "entity-2" then
        -- entity-2 listens for pings emitted by entity-1
        ping_event:subscribe(function(msg)
            print("entity-2 received", msg)
        end)
    end
end

function update(engine, self, input, dt)
    if self.name == "entity-1" then
        -- emit the event each time space is pressed
        if input:was_key_pressed('Space') then
            ping_event:emit("hello from entity-1")
        end
    end
end
```

When both objects use this script, `entity-1` will send a message whenever the
space key is pressed and `entity-2` will print the text.

### Per-Entity Events

Every entity proxy also exposes a lightweight signal system similar to Godot.
Define events on one entity and connect to them from another:

```lua
-- script on entity-1
function start(engine, self)
    self:define_event('ping')
end

function update(engine, self, input)
    if input:was_key_pressed('Space') then
        self:emit_event('ping', 'hello from entity-1')
    end
end

-- script on entity-2
local sender

function start(engine, self)
    sender = engine:find_entity_by_name('entity-1')
    if sender then
        sender:subscribe_event('ping', function(msg)
            print('entity-2 got', msg)
        end)
    end
end
```

When entity-1 emits the `ping` event, the callback registered by entity-2 runs.

### Global Events

You can also broadcast events globally so any script can react. The sender
entity is provided to listeners:

```lua
-- entity-1.lua
function start(engine, self)
    engine:define_event("ping") -- optional
end

function update(engine, self, input)
    if input:was_key_pressed("Space") then
        engine:emit_event("ping", self, "hello from entity-1")
    end
end

-- entity-2.lua
function start(engine, self)
    engine:subscribe_event("ping", function(sender, msg)
        print("entity-2 got ping from", sender:name(), ":", msg)
    end)
end
```

When `entity-1` emits the `ping` event, any subscribed callbacks receive the
emitting entity proxy and message.

Event registrations are stored per scene. Switching scenes resets the lists so
different scenes can reuse the same event names without interference.

## UI System
The engine now has two separate UI systems:

1. **Game UI System**: WebGPU-based UI for in-game interfaces (in `src/ui/`)
2. **Editor UI System**: EGUI-based editor interface (moved to `vetrace_editor` crate)

The editor UI (`MainWindow`, `SandboxWindow`, inspector panels) has been moved
to the separate `vetrace_editor` crate and is now available as an optional plugin.

`UIPanel`, `UIButton` and `UITextEditor` provide basic in-game widgets. Combine
them with `UILayout` and `UIScreenSpace` to position elements. Buttons emit a
`clicked` signal while text editors emit `changed` with the current text which
Lua scripts can subscribe to using `entity:subscribe_event`. The
`examples/ui_interaction.rs` demo shows two buttons reacting to editor input.
When creating an app, the `enable_editor` flag of `AppConfig` only toggles the
editor interface—game UI widgets remain interactive regardless of this flag.

## Rendering Pipeline
Objects in the `Scene` are converted to `GpuObject` structs and uploaded to
GPU buffers via wgpu. The renderer first rasterizes all geometry to a
GBuffer and then executes a compute shader that traces secondary rays using
that data. Finally a composite pass draws the result to the screen.

Meshes stored in Wavefront `.obj` files can be loaded by adding the `ObjMesh`
component to an entity and setting its `path` field. Triangles are imported and
uploaded automatically. The renderer also builds a Bounding Volume Hierarchy
using the [`bvh`](https://crates.io/crates/bvh) crate each frame to accelerate
ray traversal. Leveraging the external crate fixes issues when multiple meshes
or objects are present in the scene.

## Spawning Objects
Call `spawn_object` with an `Object` to add it to the scene and automatically
create ECS components:

```rust
pub fn spawn_object(&mut self, object: Object) {
    self.scene.add_object(object);
    let entity = self.world.spawn();
    let object_id = (self.scene.objects.len() - 1) as u32;
    self.world.insert(entity, ObjectRef { id: object_id });
    /* default components inserted here */
}
```
【F:src/engine/engine.rs†L409-L452】

To immediately load geometry from a Wavefront file, use `spawn_mesh_object`:

```rust
pub fn spawn_mesh_object(&mut self, path: &str, object: Object) -> Result<(), String> {
    /* loads triangles and calls `spawn_object` internally */
}
```
【F:src/engine/engine.rs†L454-L470】

## Actor Wrapper
`Actor` behaves like Unity's GameObject, wrapping an entity and providing convenience methods.

```rust
use vetrace_engine::{Actor, Engine, World};
use vetrace_engine::components::components::Transform;

let mut engine = Engine::new(false);
{
    let mut world = engine.world();
    let mut actor = world.spawn_actor("item");
    actor.add_component::<Transform>();
    if actor.has_component::<Transform>() {
        println!("Actor has a Transform");
    }
    for name in actor.list_components() {
        println!("Component: {}", name);
    }
}
```


## Engine Loop
`Engine::run_with_behaviour` drives the main loop, updating behaviours, scripts
and rendering each frame.

```rust
while self.running {
    let delta = (now - last_frame_time).as_secs_f32();
    behaviour.update(self, delta);
    /* update registered behaviours and scripts */
    self.renderer.render(&render_params);
    self.window.swap_buffers();
}
```
【F:src/engine/engine.rs†L165-L229】

## Extending the Engine
- **Add components** using `auto_register_component` so they appear in the
  inspector.
- **Create behaviours** in Rust by implementing `Behaviour` and registering with
  `engine.add_behaviour`.
- **Create Lua behaviours** in `generated/behaviours/*.lua` and reload.
- **Add component factories** to parse custom data when loading scenes.

## Project Structure
```
src/
  engine/        - Engine orchestration
  components/    - Built-in components
  behaviour/     - Behaviour implementations
  scene/         - Scene graph and JSON loader
  input/         - SDL2 input wrapper
  rendering/     - wgpu renderer
  ui/            - WebGPU-based game UI system
  app/           - Application framework and plugin system

vetrace_editor/  - Separate crate for editor functionality
  src/
    windows/     - Editor windows (MainWindow, SandboxWindow)
    inspector/   - Component inspector
    gizmo/       - 3D transform gizmos
    selection/   - Entity selection system
```

## Player Example
Define a component and Lua script controlling a player:

```rust
#[derive(Default, Debug)]
pub struct Player;
impl Component for Player {}
impl Inspectable for Player {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField> { vec![] }
}
```
【F:src/components/components.rs†L15-L23】

Register it with:
```rust
engine.auto_register_component::<Player>("Player");
```
Lua behaviour (`generated/behaviours/PlayerBehaviour.lua`):
```lua
function start(engine, self)
end

function update(engine, self, input, dt)
    -- movement using input could go here
end
```
Finally, a Rust behaviour could query input:
```rust
struct PlayerControl;
impl Behaviour for PlayerControl {
    fn update(&mut self, engine: &mut Engine, dt: f32) {
        if engine.input.is_key_down(Keycode::Space) {
            // jump or similar
        }
    }
}
```
Add `PlayerControl` via `engine.add_behaviour(PlayerControl);`

### LookAt Component
`LookAt` rotates an entity so it faces another entity or the mouse cursor.
It is registered automatically in `Engine::register_default_components`.
The `target` field stores the entity name (or `"mouse"`), while the
`rotate_x`, `rotate_y` and `rotate_z` booleans control which axes can rotate.
To make an entity continuously orient toward the mouse cursor on the Z axis:
```rust
let tracker = engine.spawn_empty("tracker");
engine.world.insert(tracker, LookAt {
    target: "mouse".into(),
    rotate_x: false,
    rotate_y: false,
    rotate_z: true,
});
engine.add_behaviour(LookAtBehaviour);
```

## Collision System
Entities with `Transform` and `Collider` components generate `CollisionEvent`s each frame.
Lua scripts may implement `on_collision(engine, self, other)` to react.
Use `other.name` or `other:has_tag("enemy")` inside Lua.
Rust code can read `engine.collision_events`.
Add `StaticBody` to an entity to block movement of other colliders on impact.
Use `KinematicBody` for scripted motion that still collides via Rapier.
Add `RevoluteJoint` to connect two bodies with a hinge.
Add `BallJoint` to connect two bodies with a freely rotating joint.
Both joint components expose `contacts_enabled` to control whether the
connected bodies collide with each other.
Helper functions:
```rust
engine.get_entity_name(entity); // -> Option<&str>
engine.entity_has_tag(entity, "enemy");
```

## Sprite3D Component
`Sprite3D` spawns a textured quad mesh in the world that is processed like any
other piece of geometry. It receives lighting, shadows and global illumination
from the raytracing pipeline. Set `facing_camera` for billboarding or leave it
`false` to orient the quad manually. Use `double_sided` if the texture should be
visible from both sides. No separate sprite renderer is required.

Load a texture and attach a sprite:
```rust
use vetrace_engine::rendering::TextureStorage;
use vetrace_engine::components::components::{Transform, Sprite3D};

let mut textures = TextureStorage::new();
let handle = textures.load_texture("assets/textures/tree.png");
// `tree.png` is not included. Place your own image in `assets/textures` before
// running the example. Textures are converted to RGBA so most common formats
// like PNG or JPEG work. A missing file triggers an `IoError` while a corrupt
// image results in a `DecodingError`.

let entity = engine.spawn_empty("tree");
engine.world.insert(entity, Transform::default());
engine.world.insert(entity, Sprite3D {
    texture: handle,
    size: [2.0, 2.0],
    facing_camera: false, // set to true for billboarding
    double_sided: true,
});
```

## Particle System

The engine includes simple CPU and GPU particle implementations. A `Particle`
component defines how each particle moves and fades. When attached to an entity
the CPU system updates its position every frame and removes the entity when
`lifetime` reaches zero. The GPU system performs the same logic through a
compute shader and synchronizes the results back to the world.

```rust
use vetrace_engine::components::components::{Particle, Transform};

let spark = engine.spawn_empty("spark");
engine.world.insert(spark, Transform::default());
engine.world.insert(spark, Particle {
    velocity: [0.0, 2.0, 0.0],
    lifetime: 1.0,
    start_size: 0.2,
    end_size: 0.0,
    looping: false,
    ..Default::default()
});
```

### Lerp Component

Attach a `Lerp` component (typically the `F32` variant) alongside a particle to animate its size or other values.
`Lerp` stores a start and end value, progress and speed. When its `state` is
`PlayingForward` the `progress` increases each update; other states allow
ping‑pong or looping behaviour. Both particle systems read the lerp value to
scale the particle and adjust lifetime accordingly.

```rust
engine.world.insert(spark, Lerp::F32(LerpData {
    start: 0.2,
    end: 0.0,
    progress: 0.0,
    speed: 1.0,
    loop_mode: LoopMode::None,
    state: LerpState::PlayingForward,
    easing: Easing::Linear,
}));
```

## Networking

Networking uses a lightweight UDP transport. `NetServer` accepts clients sending
`NetPacket::Ping` and assigns each one a unique `ClientId`. Components that
implement `NetSyncComponent` can be registered with a `NetSyncRegistry`. The
`NetSyncSystem` periodically serializes changed components and sends them to
clients while `ApplyComponentUpdates` applies received changes on the client.

```rust
use vetrace_engine::net::{NetServer, NetSyncRegistry};
use vetrace_engine::systems::networking::{NetSyncSystem, ServerInputSystem};

let mut server = NetServer::new("0.0.0.0:4000".parse().unwrap()).unwrap();
let registry = NetSyncRegistry::default();
engine.add_behaviour(NetSyncSystem::new(&mut server, &registry, 2));
engine.add_behaviour(ServerInputSystem { server: &mut server });
```
