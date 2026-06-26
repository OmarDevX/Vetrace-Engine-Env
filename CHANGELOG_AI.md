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
