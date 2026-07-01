# Experimental hybrid effect shaders

These WGSL files are **not active production shaders**. They were moved here to
make the active renderer path explicit: the current WGPU renderer uses the
monolithic `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` compute shader for
ray/path-traced visibility and effects, plus the active denoise and atmosphere
LUT compute shaders documented in `vetrace_engine/docs/SHADER_ARCHITECTURE.md`.

Files in this folder may be used as references when a future renderer wires a
split RT architecture, but they must not be treated as production-active until
Rust creates bind group layouts, pipelines, dispatches, resource transitions,
and tests for them.
