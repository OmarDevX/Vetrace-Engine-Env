# WGPU shader architecture

The renderer now uses a mixed architecture: monolithic path tracing remains the production path for primary-visibility path tracing, while `RendererMode::HybridEffects` uses decomposed raster G-buffer plus split RT effect compute passes and a compositor.

## Active shader map

| Mode / pass | Rust loader | Active shader(s) | Status |
| --- | --- | --- | --- |
| Raster game / bootstrap compute | `WgpuRenderer::new` builds the bootstrap compute pipeline and dispatches it for non-decomposed raster fallback. | `assets/shaders/wgpu/hybrid/bootstrap.comp.wgsl`, `assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl` | Active production fallback/lightweight compute path. |
| Hybrid effects split RT passes | `WgpuRenderer::new` allocates effect targets, completed bind layouts/groups, and pipelines; `render()` dispatches them when `RendererMode::uses_decomposed_rt_effects()` is true. | `assets/shaders/wgpu/experimental/hybrid_effects/rt_shadows.comp.wgsl`, `rt_reflections.comp.wgsl`, `rt_gi.comp.wgsl`, `rt_transparency.comp.wgsl`, `composite.comp.wgsl` | Active production for `RendererMode::HybridEffects`; still located in the experimental tree until the shared WGSL helper extraction is complete. |
| Cinematic/pathtrace mode | Same compute pipeline as game/hybrid mode; `RendererMode::CinematicPathTrace` changes uniforms/sample behavior rather than selecting a second WGSL file | `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` | Active production. |
| Raster/PBR mesh pass | `WgpuRenderer::new` creates the PBR render pipeline | `shaders/simple_pbr.wgsl` | Active production raster shader. This is the active raster contract, not `raster.frag.wgsl`. |
| Raytrace denoise | `WgpuRenderer::new` creates `rt_denoise_pipeline`; `render()` dispatches `rt_denoise` after raytrace | `assets/shaders/wgpu/hybrid/rt_denoise.comp.wgsl` | Active production. |
| Generic post denoise / temporal resolve | `WgpuRenderer::new` creates `denoise_pipeline`; `render()` dispatches `denoise` | `assets/shaders/wgpu/hybrid/denoise.comp.wgsl` | Active production. |
| Atmosphere LUTs | `WgpuRenderer::new` creates LUT pipelines and `render()` dispatches LUT updates when needed | `assets/shaders/wgpu/atmosphere/transmittance_lut.comp.wgsl`, `sky_view_lut.comp.wgsl`, `multi_scattering_lut.comp.wgsl`, `aerial_perspective_lut.comp.wgsl` | Active production support shaders. |
| SDFGI support | `WgpuRenderer::new` creates SDFGI prepass/inject/mip pipelines | `assets/shaders/wgpu/hybrid/sdfgi_prepass.comp.wgsl`, `sdfgi_inject.comp.wgsl`, `sdfgi_mips.comp.wgsl` | Active production support shaders. |
| Clouds | No WGPU pipeline currently includes `cloud_raymarch.comp.wgsl` | `assets/shaders/wgpu/clouds/cloud_raymarch.comp.wgsl` | Experimental/future. Cloud/fog/atmosphere timing currently comes from the monolithic pathtrace path and LUTs. |

## Experimental / future shaders

The following shaders remain experimental/future references in
`assets/shaders/wgpu/experimental/hybrid_effects/`:

- `raytrace.comp.wgsl` — stale duplicate of the monolithic compute path kept only as a reference.
- `raster.vert.wgsl`, `raster.frag.wgsl` — future/deferred GBuffer prototype; `simple_pbr.wgsl` is the active raster shader.

The split RT effect shaders are production-wired from this directory and should move to `assets/shaders/wgpu/hybrid/` after their shared traversal/material helpers are factored out of the monolithic pathtrace shader.

## Duplication policy

`pathtrace.comp.wgsl` remains the single production monolithic RT shader. Split hybrid-effect passes must not fork full BVH/material/shadow traversal code; shared WGSL generation/fragments should be used for scene structs, `Params`, materials, BVH traversal, shadows, GI, clouds, and atmosphere as the split passes grow beyond screen-space/probe approximations.
