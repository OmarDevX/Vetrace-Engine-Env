# KNOWN_PATTERNS.md

## Route Patterns

Traditional web route patterns: Unknown / not detected.

This project uses native app/engine flow instead of HTTP routes.

Detected runtime entry patterns:

- App framework:
  - `app().with_title(...).with_size(...).add_plugin(...).run(MyApp)`
  - User app implements `App`.
  - App lifecycle:
    - `setup`
    - `update`
    - `render`
    - `cleanup`
    - `on_resize`
    - `on_input`

- Engine manual/runtime pattern:
  - `Engine::new(is_2d)`
  - `Engine::run`
  - `Engine::run_default`

- Examples are under:
  - `vetrace_engine/examples/*.rs`

## Controller Patterns

Traditional web controllers: Unknown / not detected.

Equivalent runtime control is spread through:

- `Engine`
  - central runtime controller/state owner.
- `App`
  - application lifecycle controller.
- `Plugin`
  - modular lifecycle controller.
- `Behaviour`
  - per-frame system/gameplay controller.
- `EditorPlugin`
  - editor controller plugin.

Important lifecycle traits:

```rust
trait App {
    fn setup(&mut self, engine: &mut Engine) {}
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}
    fn render(&mut self, engine: &mut Engine) {}
    fn cleanup(&mut self, engine: &mut Engine) {}
    fn on_resize(&mut self, engine: &mut Engine, width: u32, height: u32) {}
    fn on_input(&mut self, engine: &mut Engine, event: &InputEvent) {}
}
trait Plugin {
    fn name(&self) -> &'static str;
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>>;
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), Box<dyn std::error::Error>>;
    fn render(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>>;
    fn render_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>>;
    fn cleanup(&mut self, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>>;
}
trait Behaviour {
    fn start(&mut self, engine: &mut Engine) {}
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}
}
Model / Database Patterns

Traditional database models/migrations: Unknown / not detected.

Detected data model patterns:

ECS components are the primary data model.
Entity is an ID wrapper.
World stores components by TypeId.
Scene stores renderable objects/GPU data separately.
ObjectRef links ECS entities to scene object indices.
Metadata stores entity names/tags.
Scene JSON stores nodes and non-object entities.

Key model files:

vetrace_engine/src/components/components.rs
vetrace_engine/src/ecs/world.rs
vetrace_engine/src/scene/object.rs
vetrace_engine/src/scene/loader.rs
vetrace_engine/src/scene/scene.rs

Scene format pattern:

SceneFile {
    nodes: Vec<NodeFile>,
    entities: Vec<EntityFile>,
}

Object-backed node:

NodeFile {
    name,
    tags,
    position,
    color,
    size,
    scale,
    is_cube,
    components,
}

Non-object entity:

EntityFile {
    name,
    tags,
    components,
}

Component data:

ComponentFile {
    name,
    data: serde_json::Value,
}
Service / Helper Patterns

No service layer in web-app sense.

Reusable engine helper patterns:

Engine methods are split across multiple files using impl Engine.
High-level wrapper APIs:
Actor
Stage
World
Reusable serialization helpers:
apply_component_data
export_component_data
Reusable component registration:
register_component
auto_register_component
register_component_factory
Reusable generated component system:
GeneratedSpec
GeneratedComponent
GeneratedStorage
Reusable plugin system:
Plugin
PluginManager
Reusable event system:
EventBus
Event
Reusable network helpers:
NetSocket
NetClient
NetServer
NetSyncRegistry
Reusable render/resource helpers:
load_obj_file
generate_cube_triangles
generate_sphere_triangles
shader compile/link helpers.
View / Template Patterns

Traditional templates: Unknown / not detected.

Detected UI/view pattern is egui.

Editor UI pattern:

EditorPlugin owns:
MainWindow
SandboxWindow
InspectorPlugin
GizmoPlugin
SelectionPlugin
MainWindow::ui(ctx, sandbox, engine) renders:
top panel
left panel
right panel
file explorer
sandbox window
UI helper style:
functions like top_panel_ui, left_panel_ui, right_panel_ui, etc.
egui panels/windows/scroll areas/buttons/combo boxes/sliders.
direct engine API calls from button handlers.

Game UI component pattern:

UI represented as ECS components:
UILabel
UIPanel
UIButton
UITextEditor
UIList
UILayout
UIScreenSpace
ColorRect
JavaScript Patterns

JavaScript: Unknown / not detected.

Frontend/event patterns are Rust + egui + SDL2:

SDL2 event polling.
Input state updated by Input.
SDL events converted into egui events through sdl_event_to_egui_event.
egui callbacks/panels handle editor actions.
Plugin UI rendering through render_ui.
Error Handling Patterns

Mixed error handling detected.

Patterns:

Many public operations return Result<(), Box<dyn std::error::Error>>.
File load/save functions return Result.
Some initialization paths use unwrap/expect.
Editor button handlers often log with eprintln!.
Networking methods often return std::io::Result.
Shader/renderer setup frequently uses expect.
Some optional systems degrade gracefully:
audio initialization can print errors and disable audio.

Guideline:

For new code, prefer Result in public/load/save paths.
Avoid panics unless matching existing initialization style.
In editor UI handlers, log user-facing failures clearly.
Do not silently swallow errors unless existing pattern does.
Permission / Role Patterns

Application permissions/roles: Unknown / not detected.

Networking role enum detected:

NetRole {
    Client,
    Server,
    Offline,
}

This is networking role/state, not user authorization.

API Response Patterns

HTTP/API response format: Unknown / not detected.

Detected data exchange formats:

Scene JSON:
SceneFile
NodeFile
EntityFile
ComponentFile
Network packets serialized with bincode:
NetPacket::Ping
NetPacket::Pong
NetPacket::Disconnect
NetPacket::Connect
NetPacket::AssignEntity
NetPacket::Ack
NetPacket::Reliable
NetPacket::SpawnObject
NetPacket::DespawnObject
NetPacket::Input
NetPacket::Snapshot
NetPacket::Rpc
NetPacket::ComponentUpdate
NetPacket::ComponentBatch
NetPacket::TransformSync
NetPacket::Custom
Component scene data:
serde_json::Value.
File Upload / Asset Patterns

Traditional web file upload: Unknown / not detected.

Asset/file patterns:

Assets rooted at assets.
AssetManager::new("assets") is used during engine initialization.
Texture/model files are loaded from filesystem.
.gitignore ignores:
assets/textures/*.png
assets/models/*
.env
target
Scene files are JSON and selected through rfd::FileDialog in editor.
Generated components:
generated/components/*.rs
Generated behaviours:
generated/behaviours/*.lua
OBJ mesh loading:
spawn_mesh_object(path, object)
update_obj_meshes
glTF asset support detected through gltf dependency and asset/example files.
Naming Conventions
Crates:
vetrace_engine
vetrace_editor
vetrace_engine_macros
Rust modules/files:
snake_case.rs
folders by subsystem.
Rust types:
PascalCase
examples:
Engine
World
Entity
Transform
Renderable
PostProcessing
WgpuRenderer
EditorPlugin
Functions/methods:
snake_case
examples:
spawn_object
load_scene_from_file
register_default_components
auto_register_component
rebuild_from_world
Component registration names:
Usually PascalCase strings matching component structs.
Important alias:
ScriptComponent is registered as "Script".
Constants:
SCREAMING_SNAKE_CASE
examples:
GI_MODE_SDFGI
MAX_ATMOSPHERES
SCENE_FLAG_STATIC_GEOMETRY
Shader files:
WGSL files use descriptive lowercase names with stage suffixes:
pathtrace.comp.wgsl
denoise.comp.wgsl
sprite_vert.wgsl
sprite_frag.wgsl
OpenGL shaders use .frag, .vert, .glsl.
Scene JSON keys:
lower/snake-like names:
nodes
entities
name
tags
position
color
size
scale
is_cube
components
data
