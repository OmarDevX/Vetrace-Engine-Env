# PROJECT_MAP.md

## Project Summary

- Project type: Rust game engine / editor workspace.
- Main language(s): Rust, WGSL, GLSL, small Python validation scripts, JSON scene/config files.
- Framework(s): Cargo workspace, custom ECS, SDL2, wgpu, egui, Rapier physics, mlua Lua scripting, Kira audio, glTF asset loading.
- Architecture pattern: Custom Entity Component System + central `Engine` object + plugin/app framework + separate editor plugin crate.
- Main purpose: Experimental raytracing-capable game engine with hybrid rasterization/raytracing rendering, ECS scene management, editor UI, Lua/Rust behaviours, networking, physics, audio, particles, UI components, and asset/shader systems.

## Folder Structure

- `/Cargo.toml`
  - Root Cargo workspace.
  - Workspace members detected:
    - `vetrace_engine`
    - `vetrace_editor`
  - Note: `vetrace_engine_macros` exists and is used as a path dependency, but is not listed as a root workspace member.

- `/Cargo.lock`
  - Root workspace lockfile.

- `/APP_FRAMEWORK_GUIDE.md`
  - Documents the app framework and plugin system.

- `/MIGRATION_GUIDE.md`
  - Documents migration from older engine/editor coupling to separated app/plugin architecture.

- `/space.json`
  - Scene/config JSON file. Exact runtime role: likely scene data, but not fully verified.

- `/scripts`
  - Python validation scripts for WGSL syntax/layout contracts.
  - Important files:
    - `scripts/validate_wgsl_layouts.py`
    - `scripts/validate_wgsl_syntax.py`

- `/vetrace_engine`
  - Main engine crate.
  - Contains ECS, engine runtime, renderer, assets, scene system, components, behaviours, systems, networking, inspector, shaders, docs, examples.

- `/vetrace_engine/src`
  - Main engine source.

- `/vetrace_engine/src/lib.rs`
  - Library public API exports.
  - Re-exports `Engine`, `Actor`, `Stage`, `World`, `Renderer`, `Component`, `Entity`, asset/rendering/LOD/material types.

- `/vetrace_engine/src/main.rs`
  - Empty binary entry point: `fn main() {}`.

- `/vetrace_engine/src/app`
  - App framework.
  - Contains `App`, `AppBuilder`, plugin manager, event bus, and application loop.

- `/vetrace_engine/src/ecs`
  - Custom ECS implementation.
  - Contains `Entity`, `Component`, `World`, and `Behaviour`.

- `/vetrace_engine/src/engine`
  - High-level engine API and runtime logic.
  - Contains `Engine`, `EngineCore`, scene/object spawning, component registration, scripts, physics state, run loop, stage/actor APIs, scene manager, UI hooks, and manager wrappers.

- `/vetrace_engine/src/components`
  - Built-in engine/game/editor components.
  - Large central file: `components.rs`.
  - Generated component support: `generated.rs`.
  - Build-time generated component include: `components/mod.rs`.

- `/vetrace_engine/src/systems`
  - Behaviour systems updated by the engine.
  - Includes physics, collision, hierarchy, transform sync, selection, gizmo, audio, networking, raycast, particles, timer, animation, lerp, sprite mesh.

- `/vetrace_engine/src/rendering`
  - Rendering abstraction and implementations.
  - Contains OpenGL-era helpers and wgpu renderer.
  - Important:
    - `rendering/mod.rs`
    - `rendering/renderer.rs`
    - `rendering/resource.rs`
    - `rendering/wgpu_renderer/*`

- `/vetrace_engine/src/rendering/wgpu_renderer`
  - Main wgpu renderer implementation.
  - Important files:
    - `renderer.rs`
    - `renderer_impl.inc.rs`
    - `setup.rs`
    - `types.rs`

- `/vetrace_engine/src/scene`
  - Scene object representation, scene file loading/saving, BVH, object factory helpers.
  - Important files:
    - `scene.rs`
    - `object.rs`
    - `loader.rs`
    - `factories.rs`
    - `bvh.rs`
    - `tri_bvh.rs`

- `/vetrace_engine/src/behaviour`
  - Script and behaviour support.
  - Includes Rust behaviours and Lua behaviour bridges.
  - Important files:
    - `script.rs`
    - `component_lua.rs`
    - `rotator.rs`
    - `look_at.rs`
    - `post_processing.rs`

- `/vetrace_engine/src/net`
  - UDP networking primitives.
  - Includes packets, transport, client/server, RPC placeholder, sync registry, tick manager.

- `/vetrace_engine/src/inspector`
  - Runtime/editor inspection support.
  - Uses `Inspectable`, `ExportedField`, `ExportKind`.

- `/vetrace_engine/assets/shaders`
  - OpenGL and WGSL shader assets.
  - Important shader families:
    - OpenGL default/raster/raytracing/sprite/ui shaders.
    - WGPU hybrid path tracing, denoise, SDFGI, atmosphere, clouds, sprite, UI, postprocess shaders.

- `/vetrace_engine/assets/textures`
  - Texture assets. PNGs ignored by `.gitignore`.

- `/vetrace_engine/assets/ui_styles`
  - UI style assets. Exact role not deeply inspected.

- `/vetrace_engine/generated`
  - Runtime/generated components and behaviours area.
  - `build.rs` also reads `generated/components`.

- `/vetrace_engine/examples`
  - Example apps/demos.
  - Includes top-down shooter, networking examples, editor demo, app framework demo, PBR cat example, sprite, car, rope, audio, UI interaction, WebGPU UI demo.

- `/vetrace_editor`
  - Separate editor plugin crate.
  - Depends on `vetrace_engine`.
  - Provides egui editor windows, inspector, gizmo, selection, UI components.

- `/vetrace_editor/src`
  - Editor plugin implementation.
  - Important files:
    - `lib.rs`
    - `windows/main_window.rs`
    - `windows/sandbox_window.rs`
    - `inspector.rs`
    - `gizmo.rs`
    - `selection.rs`
    - `ui_components.rs`

- `/vetrace_editor/assets/shaders`
  - Editor-specific shader assets.
  - Detected:
    - `postprocess/outline.wgsl`

- `/vetrace_engine_macros`
  - Procedural macro crate.
  - Provides `#[derive(Inspectable)]` with `#[export]` attributes.
  - Used by `vetrace_engine`.
  - Not listed as a root workspace member.

## Main Entry Points

- Root workspace:
  - `/Cargo.toml`

- Engine crate:
  - `vetrace_engine/Cargo.toml`
  - `vetrace_engine/src/lib.rs`
  - `vetrace_engine/src/main.rs`
  - `vetrace_engine/build.rs`

- Engine construction:
  - `vetrace_engine/src/engine/init.rs`
    - `Engine::new(is_2d: bool)`
  - `vetrace_engine/src/engine/run.rs`
    - `Engine::run`
    - `Engine::run_default`

- App framework:
  - `vetrace_engine/src/app/mod.rs`
    - `App`
    - `AppBuilder`
    - `app()`
    - `AppBuilder::run`

- Plugin system:
  - `vetrace_engine/src/app/plugin.rs`
    - `Plugin`
    - `PluginManager`

- Editor plugin:
  - `vetrace_editor/src/lib.rs`
    - `EditorPlugin`
    - `editor()`

- Rendering:
  - `vetrace_engine/src/rendering/mod.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/renderer.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/setup.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/types.rs`

- Scene loading/saving:
  - `vetrace_engine/src/scene/loader.rs`
  - `vetrace_engine/src/engine/objects.rs`
    - `load_scene`
    - `load_scene_from_file`
    - `save_scene_to_file`

- ECS:
  - `vetrace_engine/src/ecs/world.rs`
  - `vetrace_engine/src/ecs/entity.rs`
  - `vetrace_engine/src/ecs/component.rs`
  - `vetrace_engine/src/ecs/behaviour.rs`

- Components:
  - `vetrace_engine/src/components/components.rs`
  - `vetrace_engine/src/components/generated.rs`
  - `vetrace_engine/src/components/mod.rs`

- Examples:
  - `vetrace_engine/examples/*.rs`
  - `vetrace_engine/examples/pbr_cat/main.rs`

## Backend Flow

This is not a traditional web backend with routes/controllers. The runtime flow is engine/application based.

Typical app framework flow:

1. User creates an app implementing `vetrace_engine::app::App`.
2. User builds app with `app().with_title(...).with_size(...).add_plugin(...).run(MyApp)`.
3. `AppBuilder::run` creates `Engine::new(false)`.
4. Engine initializes SDL2 window, renderer, ECS world, input, physics, egui context, scene, assets, default factories, default components, generated components, Lua scripts, and behaviours.
5. Plugins are registered and initialized through `PluginManager`.
6. `app.setup(&mut engine)` runs.
7. Main loop:
   - Begin input frame.
   - Poll SDL2 events.
   - Convert relevant SDL events to egui events.
   - Dispatch input callbacks.
   - Update plugins.
   - Update app logic.
   - Update engine behaviours/systems.
   - Update scripts/component behaviours.
   - Rebuild scene/render data as needed.
   - Render frame.
   - Render egui/editor/plugin UI.
8. Shutdown calls cleanup paths.

Typical entity/object flow:

1. Object spawned through `Engine::spawn_object`, `spawn_cube`, `spawn_sphere`, `spawn_mesh_object`, `spawn_empty`, `Stage::spawn_actor`, or `World::spawn_actor`.
2. `Engine` creates an ECS `Entity`.
3. For renderable objects, `ObjectRef`, `Metadata`, `Transform`, `Renderable`, `Collider`, `Material`, `AngularVelocity`, and `Shape` may be inserted.
4. `EngineCore` maps scene object index to entity.
5. Systems update ECS components.
6. `Scene::rebuild_from_world` converts ECS state into GPU object/material/light/cloud/atmosphere data.
7. Renderer uploads buffers/textures and runs render passes/compute pipelines.

Scene file flow:

1. `scene::loader::load_scene(path)` parses JSON into `SceneFile`.
2. `Engine::load_scene` iterates `nodes` and `entities`.
3. For nodes:
   - Creates `Object`.
   - Spawns object.
   - Applies metadata.
   - Applies components through registered component factories or generated component storage.
4. For non-object entities:
   - Spawns empty entity.
   - Applies metadata and components.
5. `Engine::save_scene_to_file` rebuilds world/scene state and serializes back to JSON.

## Frontend Flow

There is no HTML/web frontend. UI is native/editor/game UI through egui.

Editor UI flow:

1. `vetrace_editor::EditorPlugin` implements `Plugin` and `EditorUIRenderer`.
2. `EditorPlugin::initialize` initializes:
   - `InspectorPlugin`
   - `GizmoPlugin`
   - `SelectionPlugin`
   - `MainWindow`
   - `SandboxWindow`
3. `EditorPlugin::render_ui` calls `render_full_editor_ui`.
4. `MainWindow::ui` renders:
   - top toolbar/panel
   - left entity/scene panel
   - right inspector panel
   - file explorer
   - optional sandbox window
5. Editor actions call engine APIs directly:
   - load/save scene
   - pause/resume/restart
   - duplicate/delete entities
   - edit components
   - add generated/custom components
6. Gizmo and selection logic interact with `Engine`, selected entities, and transform components.

Game UI flow:

1. UI components exist in `vetrace_engine/src/components/components.rs`.
2. Engine UI hooks are in `vetrace_engine/src/engine/ui.rs`.
3. UI rendering can be done through egui callbacks and `EditorUIRenderer`/game UI systems.
4. UI components include:
   - `UIScreenSpace`
   - `UILabel`
   - `UIPanel`
   - `UIButton`
   - `UITextEditor`
   - `UIList`
   - `UILayout`
   - `ColorRect`

Shader/render UI flow:

1. Renderer uses WGSL and GLSL assets from `assets/shaders`.
2. WGPU renderer uses hybrid compute/raster/postprocess passes.
3. Editor has an outline postprocess shader under `vetrace_editor/assets/shaders`.

## Database / Models

- Traditional database: Unknown / not detected.
- ORM/models/migrations: Unknown / not detected.
- Persistent storage detected:
  - JSON scene files through `SceneFile`, `NodeFile`, `EntityFile`, `ComponentFile`.
  - Generated Rust component files under `generated/components`.
  - Lua behaviour scripts under `generated/behaviours`.
  - Asset files under `assets`.
- Data access pattern:
  - ECS `World` stores components in type-erased maps keyed by `TypeId`.
  - Scene objects are stored separately in `Scene.objects`.
  - `EngineCore.object_entity_map` maps scene object IDs to ECS entities.
  - Components are serialized/deserialized through `Inspectable` fields and `serde_json::Value`.
- Important data files/types:
  - `vetrace_engine/src/scene/loader.rs`
  - `vetrace_engine/src/scene/scene.rs`
  - `vetrace_engine/src/scene/object.rs`
  - `vetrace_engine/src/ecs/world.rs`
  - `vetrace_engine/src/components/components.rs`
  - `vetrace_engine/src/components/generated.rs`
  - `vetrace_engine/src/engine/component_io.rs`

## API / External Services

- HTTP/REST API: Unknown / not detected.
- External web services: Unknown / not detected.
- Local file APIs:
  - Scene JSON load/save.
  - Asset loading.
  - Texture/model/shader file loading.
  - Generated component/behaviour file creation.
  - File dialogs via `rfd`.
- GPU APIs:
  - wgpu/WebGPU.
  - OpenGL/gl legacy or fallback shader/resource helpers.
- Window/input APIs:
  - SDL2.
- Scripting:
  - Lua through `mlua`.
  - Lua behaviours under `generated/behaviours`.
  - Script loading/reloading through `Engine::reload_scripts` and component behaviour reload paths.
- Networking:
  - UDP socket wrapper in `vetrace_engine/src/net/transport.rs`.
  - Client/server primitives:
    - `NetClient`
    - `NetServer`
    - `NetSocket`
  - Packet protocol:
    - `NetPacket`
    - `ClientInfo`
    - `InputData`
    - `EntitySnapshot`
  - RPC placeholder:
    - `RpcTable`
  - Component sync:
    - `NetSyncComponent`
    - `NetSyncRegistry`
    - `collect_snapshots`
    - `apply_snapshots`
- Authentication/tokens:
  - Unknown / not detected.

## Important Modules

### ECS

Files:
- `vetrace_engine/src/ecs/entity.rs`
- `vetrace_engine/src/ecs/component.rs`
- `vetrace_engine/src/ecs/world.rs`
- `vetrace_engine/src/ecs/behaviour.rs`

Purpose:
- Entity IDs.
- Component trait.
- TypeId-based component storage.
- Query helpers.
- Behaviour trait.

Key concepts:
- `Entity(pub u32)`
- `Component`
- `World`
- `Behaviour`

### Engine Runtime

Files:
- `vetrace_engine/src/engine/engine.rs`
- `vetrace_engine/src/engine/init.rs`
- `vetrace_engine/src/engine/run.rs`
- `vetrace_engine/src/engine/objects.rs`
- `vetrace_engine/src/engine/components.rs`
- `vetrace_engine/src/engine/scripts.rs`
- `vetrace_engine/src/engine/access.rs`
- `vetrace_engine/src/engine/core.rs`
- `vetrace_engine/src/engine/physics.rs`

Purpose:
- Central runtime state and APIs.
- Object/entity creation.
- Component registration.
- Script loading.
- Scene load/save.
- Physics state.
- Rendering bridge.

Key concept:
- `Engine` is the central object. Most systems mutate/read through `&mut Engine`.

### App Framework / Plugins

Files:
- `vetrace_engine/src/app/mod.rs`
- `vetrace_engine/src/app/plugin.rs`
- `vetrace_engine/src/app/events.rs`

Purpose:
- Modern app-style runner.
- Plugin system.
- Event bus.

Key concepts:
- `App`
- `AppBuilder`
- `Plugin`
- `PluginManager`
- `EventBus`

### Actor / Stage / World High-Level API

Files:
- `vetrace_engine/src/engine/actor.rs`
- `vetrace_engine/src/engine/stage.rs`
- `vetrace_engine/src/engine/world.rs`

Purpose:
- Higher-level wrappers for entity/component access and scene interaction.
- Similar convenience APIs exist in several places; avoid duplicating them.

Key concepts:
- `Actor<'a>`
- `Stage<'a>`
- `World<'a>`

### Components

Files:
- `vetrace_engine/src/components/components.rs`
- `vetrace_engine/src/components/generated.rs`
- `vetrace_engine/src/components/mod.rs`

Purpose:
- Built-in engine components.
- Runtime/editor-inspectable component data.
- Generated component support.

Detected component groups:
- Transform/scene:
  - `Transform`
  - `GlobalTransform`
  - `Parent`
  - `Children`
  - `Metadata`
  - `ObjectRef`
- Rendering/material:
  - `Renderable`
  - `Material`
  - `Shape`
  - `ObjMesh`
  - `Sprite3D`
  - `Bloom`
  - `DepthOfField`
  - `VolumetricFog`
  - `VolumetricCloud`
  - `Atmosphere`
  - `DirectionalLight`
  - `PostProcessing`
  - `ColorRect`
- Physics/collision:
  - `Collider`
  - `StaticBody`
  - `KinematicBody`
  - `RigidBody3D`
  - `RevoluteJoint`
  - `BallJoint`
- Movement/gameplay:
  - `Velocity`
  - `AngularVelocity`
  - `Rotate`
  - `Player`
  - `LookAt`
  - `Lifetime`
  - `Particle`
  - `Raycast`
  - `Timer`
  - `Lerp`
  - `Animation`
- UI:
  - `UIScreenSpace`
  - `UILabel`
  - `UIPanel`
  - `UIButton`
  - `UITextEditor`
  - `UIList`
  - `UILayout`
- Audio:
  - `AudioSource`
- Networking:
  - `InputBuffer`
  - `UnreliableSync`
- Scripting:
  - `ScriptComponent`

### Inspector / Component Export

Files:
- `vetrace_engine/src/inspector/mod.rs`
- `vetrace_engine/src/inspector/export.rs`
- `vetrace_engine/src/inspector/fields.rs`
- `vetrace_engine_macros/src/lib.rs`

Purpose:
- Expose component fields to egui/editor.
- Apply/export component data.
- Generate `Inspectable` implementations via proc macro.

Key concepts:
- `Inspectable`
- `ExportKind`
- `ExportedField`
- `#[derive(Inspectable)]`
- `#[export]`

### Scene

Files:
- `vetrace_engine/src/scene/scene.rs`
- `vetrace_engine/src/scene/object.rs`
- `vetrace_engine/src/scene/loader.rs`
- `vetrace_engine/src/scene/factories.rs`
- `vetrace_engine/src/scene/bvh.rs`
- `vetrace_engine/src/scene/tri_bvh.rs`

Purpose:
- Runtime render scene.
- GPU structs.
- Scene JSON.
- BVH acceleration structures.
- Primitive/object factories.

Key concepts:
- `Scene`
- `Object`
- `GpuObject`
- `GpuTriangle`
- `GpuMaterial`
- `GpuCustomMaterial`
- `SceneFile`
- `NodeFile`
- `EntityFile`
- `ComponentFile`

### Rendering

Files:
- `vetrace_engine/src/rendering/mod.rs`
- `vetrace_engine/src/rendering/renderer.rs`
- `vetrace_engine/src/rendering/resource.rs`
- `vetrace_engine/src/rendering/texture.rs`
- `vetrace_engine/src/rendering/ssbo.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/*`

Purpose:
- Renderer abstraction.
- WGPU renderer.
- GPU resources.
- Shader params and post-FX uniforms.
- Texture handling.
- OBJ loading and triangle generation.

Important:
- With `wgpu` feature enabled, `rendering::Renderer` exports `WgpuRenderer`.
- With non-wgpu path, legacy `renderer::Renderer` is used.

### Shaders

Files:
- `vetrace_engine/assets/shaders/opengl/*`
- `vetrace_engine/assets/shaders/wgpu/*`
- `vetrace_editor/assets/shaders/wgpu/postprocess/outline.wgsl`

Purpose:
- OpenGL legacy/default/raytracing shader paths.
- WGPU hybrid rendering:
  - path tracing
  - denoise
  - RT denoise
  - SDFGI prepass/inject/mips
  - atmosphere LUTs
  - clouds
  - sprite
  - UI blur
  - postprocess blit

### Systems

Files:
- `vetrace_engine/src/systems/*.rs`

Purpose:
- Runtime behaviours added by `Engine::register_default_behaviours`.

Default behaviours detected:
- `Rotator`
- `CollisionSystem`
- `RapierPhysicsSystem`
- `TransformSyncSystem`
- `HierarchySystem`
- `AudioSystem`
- `SelectionSystem`
- `GizmoSystem`
- `RaycastSystem`
- `PostProcessBehaviour`
- `CpuParticleSystem`
- `LerpSystem`
- `TimerSystem`
- `AnimationSystem`
- `SpriteMeshSystem` when `wgpu` feature is enabled.

### Editor

Files:
- `vetrace_editor/src/lib.rs`
- `vetrace_editor/src/windows/main_window.rs`
- `vetrace_editor/src/windows/sandbox_window.rs`
- `vetrace_editor/src/inspector.rs`
- `vetrace_editor/src/gizmo.rs`
- `vetrace_editor/src/selection.rs`
- `vetrace_editor/src/ui_components.rs`

Purpose:
- Separate editor plugin.
- Scene/entity UI.
- Component inspector.
- Gizmo manipulation.
- Selection.
- Sandbox/object creation UI.
- File explorer.

### Networking

Files:
- `vetrace_engine/src/net/*`

Purpose:
- UDP client/server.
- Net packets.
- Reliable wrapper/ack.
- Tick manager.
- Component synchronization hooks.
- RPC table placeholder.

### Assets

Files:
- `vetrace_engine/src/assets.rs`
- `vetrace_engine/src/gpu.rs`
- `vetrace_engine/src/materials.rs`

Purpose:
- Asset loading.
- GPU texture/mesh handles.
- PBR material representation.
- glTF/texture/animation/morph target support.

## Existing Reusable Helpers / Utilities

Use these before creating new logic:

- ECS:
  - `World::spawn`
  - `World::insert`
  - `World::get`
  - `World::get_mut`
  - `World::has`
  - `World::remove`
  - `World::query`
  - `World::query_mut`
  - `World::query2`
  - `World::query2_mut`
  - `World::query3`
  - `World::query3_mut`
  - `World::query4_mut`
  - likely more query helpers below inspected snippet.

- Engine object/entity APIs:
  - `Engine::spawn_empty`
  - `Engine::spawn_object`
  - `Engine::spawn_object_as_actor`
  - `Engine::spawn_mesh_object`
  - `Engine::spawn_with_triangles`
  - `Engine::spawn_cube`
  - `Engine::spawn_sphere`
  - `Engine::delete_entity`
  - `Engine::duplicate_entity`
  - `Engine::rename_entity`
  - `Engine::get_entity_name`
  - `Engine::entity_has_tag`
  - `Engine::find_entity_by_name`
  - `Engine::find_actor_by_name`

- Scene APIs:
  - `scene::loader::load_scene`
  - `scene::loader::save_scene`
  - `Engine::load_scene`
  - `Engine::load_scene_from_file`
  - `Engine::save_scene_to_file`
  - `Engine::clear_scene`
  - `Scene::rebuild_from_world`
  - `Scene::update`

- Component APIs:
  - `Engine::register_component`
  - `Engine::auto_register_component`
  - `Engine::register_component_factory`
  - `Engine::register_default_components`
  - `Engine::add_component_entity`
  - `Engine::remove_component_entity`
  - `Engine::get_component_mut_entity`
  - `Engine::access_component_mut`
  - `Engine::list_components_entity`
  - `Engine::add_generated_component`
  - `Engine::remove_generated_component`
  - `Engine::get_generated_component_mut`
  - `Engine::remove_component_by_name`

- Component serialization:
  - `apply_component_data`
  - `export_component_data`

- High-level wrappers:
  - `Actor`
  - `Stage`
  - `engine::world::World`

- Plugin/app:
  - `App`
  - `AppBuilder`
  - `Plugin`
  - `PluginManager`
  - `EventBus`

- Rendering/resources:
  - `rendering::resource::load_obj_file`
  - `generate_cube_triangles`
  - `generate_sphere_triangles`
  - `compile_shader`
  - `link_program`
  - `TextureStorage`
  - `TextureHandle`

- Math:
  - `math.rs`
  - Detected helpers include `look_at`, `perspective`, `vec3_to_array`, `array_to_vec3`.

- Networking:
  - `NetClient`
  - `NetServer`
  - `NetSocket`
  - `NetPacket`
  - `TickManager`
  - `NetSyncRegistry`
  - `register_sync_component`
  - `collect_snapshots`
  - `apply_snapshots`

- Editor:
  - `EditorPlugin`
  - `MainWindow`
  - `SandboxWindow`
  - `InspectorPlugin`
  - `GizmoPlugin`
  - `SelectionPlugin`
  - reusable UI helpers in `ui_components.rs`.

## Authentication / Authorization

- Authentication: Unknown / not detected.
- Authorization/roles/permissions: Unknown / not detected.
- Runtime user/login/session system: Unknown / not detected.
- Networking includes client/server identity concept through `ClientId` and `ClientInfo`, but not authentication.

## State / Session / Cache

- Main runtime state:
  - `Engine`
  - `World`
  - `Scene`
  - `EngineCore`
  - `PhysicsState`
  - `Input`
  - `WindowManager`
  - `AssetManager`
  - renderer state.

- ECS state:
  - Stored in `World.components` using `TypeId -> HashMap<Entity, Component>`.
  - Entities stored in `World.entities`.

- Scene/render state:
  - `Scene.objects`
  - `Scene.gpu_objects`
  - `Scene.triangles`
  - BVH nodes
  - materials
  - atmospheres/clouds
  - `bvh_dirty`

- Engine object/entity mapping:
  - `EngineCore.object_entity_map`.

- Input state:
  - `Input` collects SDL2 event state per frame.
  - `begin_frame` clears per-frame input state in app loop.

- Script state:
  - `Engine.script_library`
  - `Engine.component_behaviours`
  - `Engine.started_scripts`
  - Lua generated behaviours under `generated/behaviours`.

- Component registration state:
  - `component_factories`
  - `component_adders`
  - `component_removers`
  - `component_editors`
  - `component_checkers`
  - `component_accessors`
  - `generated_components`
  - `generated_specs`

- Renderer cache/state:
  - WGPU renderer has many buffers/textures/pipelines/bind groups.
  - Engine has WGPU material caches:
    - `cached_gpu_materials`
    - `cached_tex_handles`
    - `cached_custom_materials`
    - `cached_custom_names`
    - `cached_shader_defs`
    - `materials_dirty`

- Scene manager:
  - `SceneManager`
  - exact multi-scene lifecycle not deeply inspected.

- Persistent app session/cache:
  - Unknown / not detected beyond file-based scenes/generated files/assets.

## Build / Run / Test Commands

Detected or strongly implied commands:

```sh
cargo check --workspace
cargo build --workspace
cargo run -p vetrace_engine
cargo run --example app_framework_demo -p vetrace_engine
cargo run --example editor_demo -p vetrace_engine
cargo run --example top_down_shooter -p vetrace_engine
cargo run --example pbr_cat -p vetrace_engine
cargo run --example webgpu_ui_demo -p vetrace_engine
python3 scripts/validate_wgsl_layouts.py
python3 scripts/validate_wgsl_syntax.py
