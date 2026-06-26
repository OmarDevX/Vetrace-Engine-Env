# AI_RULES.md

## Core Rules

- Do not edit before inspecting relevant existing files.
- Do not create duplicate functions, classes, controllers, helpers, components, systems, shaders, wrappers, or scene APIs.
- Always search for existing similar logic first.
- Reuse existing engine/component/system/editor patterns before introducing new ones.
- Keep changes minimal and focused.
- Do not rewrite architecture unless explicitly requested.
- Do not introduce new dependencies unless explicitly approved.
- Do not rename public APIs, component registration names, scene JSON fields, shader struct fields, config keys, or response/data formats unless required.
- Do not remove code unless you confirm it is unused, broken, or explicitly targeted.
- Do not leave dead code, commented-out old code, unused imports, temporary debug statements, or broad formatting noise.
- Do not treat this like a web MVC project. It is a Rust game engine/editor workspace.
- Do not invent missing facts. If unknown, say `Unknown / not detected`.

## Before Any Task

Future AI must:

1. Restate the task in concrete terms.
2. Identify the relevant subsystem:
   - ECS
   - Engine runtime
   - Components
   - Scene load/save
   - Rendering/wgpu/shaders
   - Editor plugin
   - App/plugin framework
   - Lua scripting
   - Physics
   - Networking
   - Audio
   - UI
   - Assets
3. Inspect relevant existing files.
4. Search for existing similar implementation.
5. Explain the current flow.
6. Explain the minimal change plan.
7. Warn about duplicate-code risks.
8. Only then edit.

## Required Searches Before Editing

Depending on the task, search these areas first:

- Component tasks:
  - `vetrace_engine/src/components/components.rs`
  - `vetrace_engine/src/components/generated.rs`
  - `vetrace_engine/src/engine/components.rs`
  - `vetrace_engine/src/engine/component_io.rs`
  - `vetrace_engine/src/engine/objects.rs`

- Entity/object/spawn tasks:
  - `vetrace_engine/src/engine/objects.rs`
  - `vetrace_engine/src/engine/stage.rs`
  - `vetrace_engine/src/engine/world.rs`
  - `vetrace_engine/src/engine/actor.rs`
  - `vetrace_engine/src/engine/core.rs`

- Editor UI tasks:
  - `vetrace_editor/src/lib.rs`
  - `vetrace_editor/src/windows/main_window.rs`
  - `vetrace_editor/src/windows/sandbox_window.rs`
  - `vetrace_editor/src/inspector.rs`
  - `vetrace_editor/src/gizmo.rs`
  - `vetrace_editor/src/selection.rs`
  - `vetrace_editor/src/ui_components.rs`

- Rendering/GPU tasks:
  - `vetrace_engine/src/rendering/mod.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/renderer.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/setup.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/types.rs`
  - `vetrace_engine/src/scene/object.rs`
  - `vetrace_engine/assets/shaders/wgpu`
  - `scripts/validate_wgsl_layouts.py`

- Scene save/load tasks:
  - `vetrace_engine/src/scene/loader.rs`
  - `vetrace_engine/src/engine/objects.rs`
  - `vetrace_engine/src/engine/component_io.rs`
  - `vetrace_engine/src/components/components.rs`

- Lua/script tasks:
  - `vetrace_engine/src/behaviour/script.rs`
  - `vetrace_engine/src/behaviour/component_lua.rs`
  - `vetrace_engine/src/engine/scripts.rs`
  - `generated/behaviours`

- Networking tasks:
  - `vetrace_engine/src/net`
  - `vetrace_engine/src/systems/networking.rs`
  - `vetrace_engine/src/systems/unreliable.rs`
  - networking-related components in `components.rs`

- Physics/collision tasks:
  - `vetrace_engine/src/systems/rapier_physics.rs`
  - `vetrace_engine/src/systems/collision.rs`
  - physics/collider/body components in `components.rs`
  - `vetrace_engine/src/engine/physics.rs`

- App/plugin tasks:
  - `vetrace_engine/src/app/mod.rs`
  - `vetrace_engine/src/app/plugin.rs`
  - `vetrace_engine/src/app/events.rs`
  - `vetrace_editor/src/lib.rs`

## During Editing

- Modify the smallest number of files.
- Follow existing style.
- Prefer extending existing helpers/services/components/systems.
- Keep engine/editor separation intact.
- Keep core engine independent from editor-specific state.
- Keep controllers thin does not apply; instead, keep `Engine` changes minimal and prefer existing modules.
- Keep frontend changes close to existing egui patterns.
- Preserve backward compatibility for examples and public exports.
- Preserve existing component registration names unless explicitly changing scene compatibility.
- Preserve scene JSON compatibility when possible.
- Preserve shader/Rust buffer layout compatibility.
- Do not add a new ECS storage/query system unless explicitly requested.
- Do not add a second rendering abstraction unless explicitly requested.
- Do not add a second script system unless explicitly requested.
- Do not add a second editor selection/gizmo system before reconciling existing ones.

## After Editing

Future AI must return:

1. Changed files list.
2. Why each file changed.
3. Unified diff or changed code blocks only.
4. Any commands/tests/checks that should be run.
5. Any risks or manual verification steps.
6. Whether any public API, scene format, shader layout, or component registration name changed.
7. Whether `CHANGELOG_AI.md` needs an entry.

## Forbidden Behavior

- Do not create a new component if an equivalent component already exists.
- Do not create a new helper if an equivalent helper exists.
- Do not create a new API wrapper if one already exists.
- Do not duplicate scene serialization logic.
- Do not duplicate component inspection/export logic.
- Do not duplicate selection/picking logic without checking both engine and editor selection files.
- Do not duplicate gizmo logic without checking both engine and editor gizmo files.
- Do not duplicate validation logic.
- Do not duplicate networking sync logic.
- Do not duplicate permission/auth logic; none was detected.
- Do not create parallel UI logic when an existing egui component/panel pattern exists.
- Do not make broad formatting-only changes mixed with functional changes.
- Do not directly edit generated build output under Cargo `OUT_DIR`.
- Do not assume generated component files exist unless present in the uploaded code.
- Do not blindly modify `components.rs` without checking registration and scene serialization.
- Do not modify GPU structs without checking WGSL shader structs.
- Do not delete entities inside mutable query loops unless the existing code safely supports it.
- Do not move editor logic back into `Engine`; migration docs indicate editor separation was intentional.

## Rust-Specific Rules

- Prefer compiling mentally against Rust borrow rules before proposing code.
- When mutating ECS entities while iterating:
  - collect `Entity` IDs first.
  - mutate after the query loop.
- Respect `Send + Sync` bound on `Component`.
- Respect lifetimes in `Actor<'a>`, `Stage<'a>`, and wrapper APIs.
- Be careful with unsafe code in:
  - `ecs/world.rs`
  - `inspector/mod.rs`
  - `engine/component_io.rs`
  - Lua engine proxy code.
- Avoid adding unnecessary `clone()` calls on large scene/mesh/shader data unless needed.
- Avoid changing Rust/WGSL struct field order without validation.

## Renderer / Shader Rules

- Before editing shader buffer structs, inspect both Rust and WGSL layout.
- For `GpuObject`, `GpuMaterial`, `GpuTriangle`, `ShaderParams`, `PostFxUniforms`, and similar structs:
  - keep `#[repr(C)]`.
  - keep padding fields intentional.
  - update validation script if layout contract intentionally changes.
- Run:
  - `python3 scripts/validate_wgsl_layouts.py`
- If `naga` is available, run:
  - `python3 scripts/validate_wgsl_syntax.py`

## Component Rules

- For a new built-in component:
  1. Add struct/enum in `components.rs` only if no existing component fits.
  2. Implement `Component`.
  3. Implement `Default` if it will be added via editor/default registration.
  4. Implement `Inspectable` if editor editing/scene serialization needs it.
  5. Register it in `Engine::register_default_components` if it should be editor-addable/scene-loadable.
  6. Add accessors only if existing auto registration is insufficient.
  7. Ensure scene load/save works through factories/access_component_mut.

- For generated/runtime components:
  - Use existing generated component mechanism.
  - Do not bypass `GeneratedStorage`, `GeneratedSpec`, or `build.rs`.

## Scene Rules

- Object-backed entities must keep `ObjectRef` consistent with `Scene.objects`.
- Non-object entities should not get `ObjectRef`.
- Before changing save/load format, inspect:
  - `SceneFile`
  - `NodeFile`
  - `EntityFile`
  - `ComponentFile`
  - `Engine::load_scene`
  - `Engine::save_scene_to_file`
- Scene save must not accidentally serialize internal-only components unless intended.
- Avoid breaking old scene JSON unless explicitly requested.

## Editor Rules

- Editor-specific UI belongs in `vetrace_editor`, not `vetrace_engine`.
- Main editor window logic is in `vetrace_editor/src/windows/main_window.rs`.
- Sandbox object creation/UI belongs in `vetrace_editor/src/windows/sandbox_window.rs`.
- Selection/gizmo changes should prefer the editor plugin versions unless task targets legacy compatibility systems.
- Keep `EditorPlugin` lifecycle consistent with `Plugin`.

## Changelog Rule

After an actual code change, add an entry to `CHANGELOG_AI.md` with:

- Summary
- Files changed
- Existing pattern reused
- Duplicate code avoided
- Tests/checks
- Notes
