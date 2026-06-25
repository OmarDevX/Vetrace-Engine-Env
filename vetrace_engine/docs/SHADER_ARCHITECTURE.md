# WGPU shader architecture

The renderer currently follows **Option B: active monolithic ray/path tracing**.
Disconnected split RT prototypes are kept under `assets/shaders/wgpu/experimental/`
so they cannot be mistaken for production-active files.

## Active shader map

| Mode / pass | Rust loader | Active shader(s) | Status |
| --- | --- | --- | --- |
| Game / hybrid primary 3D rendering | `WgpuRenderer::new` builds `RaytraceShaderCompiler` with `base_shader_template` and dispatches `raytrace` in `render()` | `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` | Active production monolithic compute shader for primary visibility, RT shadows/reflections/GI/transparency toggles, sky/atmosphere integration, and material logic. |
| Cinematic/pathtrace mode | Same compute pipeline as game/hybrid mode; `RendererMode::CinematicPathTrace` changes uniforms/sample behavior rather than selecting a second WGSL file | `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` | Active production. |
| Raster/PBR mesh pass | `WgpuRenderer::new` creates the PBR render pipeline | `shaders/simple_pbr.wgsl` | Active production raster shader. This is the active raster contract, not `raster.frag.wgsl`. |
| Raytrace denoise | `WgpuRenderer::new` creates `rt_denoise_pipeline`; `render()` dispatches `rt_denoise` after raytrace | `assets/shaders/wgpu/hybrid/rt_denoise.comp.wgsl` | Active production. |
| Generic post denoise / temporal resolve | `WgpuRenderer::new` creates `denoise_pipeline`; `render()` dispatches `denoise` | `assets/shaders/wgpu/hybrid/denoise.comp.wgsl` | Active production. |
| Atmosphere LUTs | `WgpuRenderer::new` creates LUT pipelines and `render()` dispatches LUT updates when needed | `assets/shaders/wgpu/atmosphere/transmittance_lut.comp.wgsl`, `sky_view_lut.comp.wgsl`, `multi_scattering_lut.comp.wgsl`, `aerial_perspective_lut.comp.wgsl` | Active production support shaders. |
| SDFGI support | `WgpuRenderer::new` creates SDFGI prepass/inject/mip pipelines | `assets/shaders/wgpu/hybrid/sdfgi_prepass.comp.wgsl`, `sdfgi_inject.comp.wgsl`, `sdfgi_mips.comp.wgsl` | Active production support shaders. |
| Clouds | No WGPU pipeline currently includes `cloud_raymarch.comp.wgsl` | `assets/shaders/wgpu/clouds/cloud_raymarch.comp.wgsl` | Experimental/future. Cloud/fog/atmosphere timing currently comes from the monolithic pathtrace path and LUTs. |

## Experimental / future shaders

The following shaders are intentionally **not** production-active and live in
`assets/shaders/wgpu/experimental/hybrid_effects/`:

- `raytrace.comp.wgsl` — stale duplicate of the monolithic compute path kept only as a reference.
- `rt_shadows.comp.wgsl`, `rt_reflections.comp.wgsl`, `rt_gi.comp.wgsl`, `rt_transparency.comp.wgsl` — future split RT effect passes.
- `composite.comp.wgsl` — future split-effect compositor.
- `raster.vert.wgsl`, `raster.frag.wgsl` — future/deferred GBuffer prototype; `simple_pbr.wgsl` is the active raster shader.

Before moving any of these back to `assets/shaders/wgpu/hybrid/`, Rust must wire
the shader into a complete pipeline: bind group layout, pipeline object, resource
allocation, dispatch/render pass, validation, and tests.

## Duplication policy

`pathtrace.comp.wgsl` is the single production monolithic RT shader. Do not add a
second production-looking monolithic shader. Future split work should either use
shared WGSL generation/fragments for scene structs, `Params`, materials, BVH
traversal, shadows, GI, clouds, and atmosphere, or stay in the experimental tree
until fully wired.
