# TASK_WORKFLOW.md

## Standard Task Prompt Template

Task:
[Describe the exact task]

Rules:
- Use `PROJECT_MAP.md`, `AI_RULES.md`, `KNOWN_PATTERNS.md`, and `CHANGELOG_AI.md`.
- Do not edit immediately.
- First inspect relevant existing files.
- Search for existing similar logic.
- Do not duplicate code.
- Keep the diff minimal.
- Return changed files only or unified diff.
- Do not introduce dependencies unless explicitly approved.
- After editing, update `CHANGELOG_AI.md`.

Required response format:
1. Understanding of task
2. Relevant existing files found
3. Existing pattern to reuse
4. Duplicate-code risks
5. Minimal implementation plan
6. Patch/diff
7. Verification steps
8. `CHANGELOG_AI.md` entry

## Bug Fix Workflow

1. Restate the bug.
2. Identify the runtime area:
   - engine
   - ECS
   - component
   - scene load/save
   - renderer/shader
   - editor
   - physics
   - networking
   - Lua/script
   - asset loading
3. Reproduce or infer the failing flow from code.
4. Locate the smallest relevant area.
5. Search existing related code.
6. Identify the root cause.
7. Patch only necessary files.
8. Avoid unrelated cleanup.
9. Suggest verification commands.
10. Add changelog entry.

## Feature Workflow

1. Restate the feature.
2. Search for an existing similar feature.
3. Identify the existing pattern to reuse.
4. Identify files that must change.
5. Identify files that must not change.
6. Explain duplicate-code risks.
7. Add only missing logic.
8. Avoid new architecture.
9. Return minimal patch.
10. Add changelog entry.

## Refactor Workflow

1. Identify duplicated or unsafe logic.
2. Confirm all call sites.
3. Confirm public API compatibility.
4. Refactor in the smallest safe step.
5. Preserve behavior.
6. Avoid changing formatting outside touched logic.
7. Run or suggest checks.
8. Return diff and migration notes if needed.
9. Add changelog entry.

## Renderer / Shader Workflow

1. Identify whether change affects:
   - Rust GPU struct
   - WGSL struct
   - bind group layout
   - pipeline setup
   - renderer state
   - scene GPU conversion
2. Inspect Rust-side files:
   - `vetrace_engine/src/scene/object.rs`
   - `vetrace_engine/src/rendering/wgpu_renderer/types.rs`
   - `vetrace_engine/src/rendering/wgpu_renderer/renderer.rs`
   - `vetrace_engine/src/rendering/wgpu_renderer/setup.rs`
3. Inspect shader files under:
   - `vetrace_engine/assets/shaders/wgpu`
4. Keep struct field order and padding synchronized.
5. Update validation script only if the contract intentionally changes.
6. Suggest:
   - `python3 scripts/validate_wgsl_layouts.py`
   - `python3 scripts/validate_wgsl_syntax.py`
   - `cargo check --workspace`

## Component Addition Workflow

1. Search existing component names in `components.rs`.
2. Search existing registration in `engine/components.rs`.
3. Decide whether this should be:
   - built-in component
   - generated component
   - existing component extension
4. If built-in:
   - add component struct/enum
   - implement `Component`
   - implement `Default` if needed
   - implement `Inspectable` if editor/scene serialization needs it
   - register through existing registration pattern
5. Update scene serialization only if needed.
6. Avoid adding duplicate helper methods.
7. Add verification steps.

## Scene Load/Save Workflow

1. Inspect:
   - `scene/loader.rs`
   - `engine/objects.rs`
   - `engine/component_io.rs`
   - relevant component definitions
2. Confirm whether data belongs to:
   - object-backed `nodes`
   - non-object `entities`
   - component `data`
3. Preserve existing JSON fields unless explicitly changing format.
4. Reuse `apply_component_data` and `export_component_data` where possible.
5. Avoid duplicating serialization logic.
6. Add backward compatibility handling when possible.

## Editor UI Workflow

1. Identify whether the change belongs to:
   - main window
   - sandbox window
   - inspector
   - gizmo
   - selection
   - reusable UI components
2. Inspect existing editor file before editing.
3. Do not move editor state into core `Engine`.
4. Use existing egui panel/window/component style.
5. Keep engine mutations through existing engine APIs.
6. Avoid new UI state if an existing field can be extended safely.
7. Return minimal patch.

## ECS / System Workflow

1. Identify target components and systems.
2. Inspect existing query usage.
3. Avoid deleting/spawning entities inside mutable query loops.
4. If needed, collect entity IDs first, mutate later.
5. Use existing `World` query methods.
6. Avoid creating new query APIs unless required.
7. Keep systems as `Behaviour` implementations unless task targets app/plugin system.

## Lua / Scripting Workflow

1. Inspect:
   - `behaviour/script.rs`
   - `behaviour/component_lua.rs`
   - `engine/scripts.rs`
2. Check existing proxy API before adding script features.
3. Avoid exposing unsafe engine operations casually.
4. Keep generated Lua files under `generated/behaviours`.
5. Do not create a separate scripting bridge unless explicitly requested.

## Networking Workflow

1. Inspect:
   - `net/packets.rs`
   - `net/transport.rs`
   - `net/client.rs`
   - `net/server.rs`
   - `net/sync.rs`
   - `systems/networking.rs`
   - `systems/unreliable.rs`
2. Reuse `NetPacket`.
3. Reuse `NetSyncRegistry` for component sync.
4. Avoid parallel transform sync logic unless replacing existing one.
5. Preserve bincode serialization compatibility if possible.

## Verification Checklist

Use relevant commands only:

```sh
cargo check --workspace
cargo build --workspace
cargo run --example app_framework_demo -p vetrace_engine
cargo run --example editor_demo -p vetrace_engine
cargo run --example top_down_shooter -p vetrace_engine
python3 scripts/validate_wgsl_layouts.py
python3 scripts/validate_wgsl_syntax.py
