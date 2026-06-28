# CHANGELOG_AI.md

This file tracks AI-assisted changes.

## Format

### YYYY-MM-DD - Task title
- Summary:
- Files changed:
- Existing pattern reused:
- Duplicate code avoided:
- Tests/checks:
- Notes:

### 2026-06-26 - WGPU raster/hybrid primary visibility
- Summary: Added a lightweight non-cinematic raster/hybrid path that rasterizes primitive scene cubes/spheres into the existing G-buffer and composes a simple lit final image without using the cinematic pathtrace shader.
- Files changed:
  - `vetrace_engine/src/rendering/wgpu_renderer/renderer.rs`
  - `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
  - `vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl`
  - `vetrace_engine/assets/shaders/wgpu/hybrid/primitive_gbuffer.wgsl`
- Existing pattern reused: Reused existing `GpuObject`/`GpuMaterial` scene upload data, existing G-buffer targets, existing PBR render-pass target layout, and existing cube/sphere triangle generation helpers.
- Duplicate code avoided: Did not duplicate scene ownership or mesh generation logic; primitive raster mesh data is a renderer-local cache generated from existing helpers.
- Tests/checks:
  - `cargo check --workspace`
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `timeout 20 cargo run --example app_framework_demo -p vetrace_engine` timed out while compiling dependencies before runtime.
  - `timeout 20 env VETRACE_SAFE_SHADER=1 cargo run --example app_framework_demo -p vetrace_engine` timed out while compiling dependencies before runtime.
- Notes: `VETRACE_SAFE_SHADER=1` keeps using bootstrap; cinematic/pathtrace modes still compile the heavy pathtrace pipeline lazily.

### 2026-06-26 - Fix hybrid compose depth binding access
- Summary: Matched the lightweight hybrid compose shader depth storage texture access to the existing compute bind group layout to avoid WGPU pipeline validation failure at startup.
- Files changed:
  - `vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl`
  - `CHANGELOG_AI.md`
- Existing pattern reused: Kept the existing pathtrace/bootstrap compute bind group layout unchanged and adjusted only the lightweight shader declaration.
- Duplicate code avoided: No duplicate bind group layout or alternate depth texture binding was introduced.
- Tests/checks:
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `cargo check --workspace`
- Notes: This fixes the reported validation error for binding 6 on `hybrid_compose_pipeline`.

### 2026-06-26 - Present lightweight hybrid color output
- Summary: Copied the lightweight hybrid/bootstrap compute color output into the screen texture before postprocess blit so non-pathtraced modes present the composed raster/hybrid image instead of stale black.
- Files changed:
  - `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
  - `CHANGELOG_AI.md`
- Existing pattern reused: Reused existing color/screen textures and the existing final blit path instead of adding another presentation path.
- Duplicate code avoided: Kept the existing postprocess pipeline unchanged.
- Tests/checks:
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `cargo check --workspace`
- Notes: Path-traced modes are excluded from this copy because `rt_denoise` already writes the screen texture for those modes.

### 2026-06-28 - Allow hybrid color texture presentation copy
- Summary: Added `COPY_SRC` usage to the WGPU `color_tex` so the non-pathtraced hybrid/bootstrap color output can be copied into `screen_tex` before final blit without WGPU validation failure.
- Files changed:
  - `vetrace_engine/src/rendering/wgpu_renderer/setup.rs`
  - `CHANGELOG_AI.md`
- Existing pattern reused: Matched the existing screen texture copy-usage pattern while keeping the final blit path unchanged.
- Duplicate code avoided: Did not add a second presentation shader or duplicate postprocess bind groups.
- Tests/checks:
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `cargo check --workspace`
- Notes: This fixes the reported `copy_texture_to_texture` validation error for `color_tex` missing `COPY_SRC`.

### 2026-06-28 - Normalize primitive material colors for raster visibility
- Summary: Fixed primitive material generation so app-framework rendering treats `Object.color` consistently in either 0-1 or 0-255 range instead of always dividing by 255, which could make some rasterized objects effectively black/invisible.
- Files changed:
  - `vetrace_engine/src/scene/object.rs`
  - `vetrace_engine/src/engine/engine.rs`
  - `vetrace_engine/src/engine/run.rs`
  - `CHANGELOG_AI.md`
- Existing pattern reused: Reused existing `Object` material fields and the primitive material upload path used before WGPU scene upload.
- Duplicate code avoided: Added one `Object::base_color_factor()` helper and reused it from both app-framework and legacy run-loop material paths.
- Tests/checks:
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `cargo check --workspace`
- Notes: No object ID filtering or hardcoded object ID was found in the primitive raster pass; all non-mesh shaded objects in `prev_objects` are instanced.

### 2026-06-28 - Apply primitive color normalization to cached material builder
- Summary: Applied `Object::base_color_factor()` to the cached app-framework GPU material builder as well, so the materials actually uploaded to WGPU for raster/hybrid frames use the same normalized primitive colors as the non-cached helper path.
- Files changed:
  - `vetrace_engine/src/engine/engine.rs`
  - `CHANGELOG_AI.md`
- Existing pattern reused: Reused the shared `Object::base_color_factor()` helper instead of adding another color conversion path.
- Duplicate code avoided: Removed the remaining direct `[obj.color[0], obj.color[1], obj.color[2], 1.0]` primitive material conversion in the app-framework material cache builder.
- Tests/checks:
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `cargo check --workspace`
- Notes: The primitive raster submit loop still enumerates all non-mesh shaded objects; this fixes the separate material upload path that the previous color patch missed.

### 2026-06-28 - Use G-buffer coverage in lightweight hybrid compose
- Summary: Made the lightweight hybrid compose shader treat `gbuf_albedo.a` as coverage, matching the existing pathtrace raster fallback logic, and clear primitive G-buffer albedo to transparent so no-geometry pixels are unambiguous.
- Files changed:
  - `vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl`
  - `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
  - `CHANGELOG_AI.md`
- Existing pattern reused: Reused the existing pathtrace raster fallback's `depth || albedo alpha` visibility test.
- Duplicate code avoided: Kept the lightweight compose shader simple and did not add another presentation pass.
- Tests/checks:
  - `python3 scripts/validate_wgsl_layouts.py`
  - `python3 scripts/validate_wgsl_syntax.py`
  - `cargo check --workspace`
- Notes: This makes the compose step rely on explicit G-buffer coverage instead of depth alone, which is safer when raster and storage depth representations diverge.
