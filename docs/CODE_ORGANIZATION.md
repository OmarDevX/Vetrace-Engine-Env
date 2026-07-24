# Code organization

This document records the current source layout after the render-to-texture cleanup.

## Baked lighting

`vetrace_render/src/baked_lighting.rs` is the public facade. Its implementation is split into:

- `baked_lighting/config.rs` — public bake configuration and report types
- `baked_lighting/runtime_bindings.rs` — per-object lightmap/probe binding selection
- `baked_lighting/object_key.rs` — stable object hashing
- `baked_lighting/file_io.rs` — `.vlight` loading, validation, saving, and installation
- `baked_lighting/debug.rs` — runtime modes and probe debug markers

The CPU baker remains a private subsystem with focused files under
`vetrace_render/src/baked_lighting_bake/` for orchestration, configuration,
atlas operations, lightmaps, probes, lighting, and sampling.

## WGPU resources

The old catch-all `gpu_types.rs` and `lifecycle_and_cache.rs` files were removed.
GPU layouts, targets, texture resources, constants, lifecycle, cache maintenance,
and memory reporting now live in files named for their responsibilities.

## Compatibility renames

- `engine/managers.rs` became `engine/component_registry.rs`.
  `engine::managers` remains as a deprecated compatibility alias.
- `vetrace_physics/defs.rs` became `scene_definitions.rs`.
  `vetrace_physics::defs` remains as a deprecated compatibility alias.
- `backend_frame.rs` became `render_frame.rs`.
- SDL `objects.rs` became `object_rasterizer.rs`.

## Size checks

Run:

```bash
bash scripts/module_size_report.sh
```

The script reports Rust source files over the configured threshold and lists
remaining textual `include!` sites for gradual conversion to explicit modules.
