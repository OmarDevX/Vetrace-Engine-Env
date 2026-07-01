# WGPU shader architecture

The renderer now uses a mixed architecture: monolithic path tracing remains the production path for primary-visibility path tracing and the constrained real-time `RendererMode::FullRaytracing` primary path, while `RendererMode::HybridEffects` uses decomposed raster G-buffer plus split RT effect compute passes and a compositor.

For feature-by-feature implementation status and future parity guidance, see `RENDERER_PARITY.md`.

## Active shader map

| Mode / pass | Rust loader | Active shader(s) | Status |
| --- | --- | --- | --- |
| Raster game / bootstrap compute | `WgpuRenderer::new` builds the bootstrap compute pipeline and dispatches it for non-decomposed raster fallback. | `assets/shaders/wgpu/hybrid/bootstrap.comp.wgsl`, `assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl` | Active production fallback/lightweight compute path. |
| Hybrid effects split RT passes | `WgpuRenderer::new` allocates effect targets, completed bind layouts/groups, and pipelines; `render()` dispatches them when `RendererMode::uses_decomposed_rt_effects()` is true. | `assets/shaders/wgpu/hybrid/rt_shadows.comp.wgsl`, `rt_reflections.comp.wgsl`, `rt_gi.comp.wgsl`, `rt_transparency.comp.wgsl`, `rt_ao.comp.wgsl`, `hybrid_effects_composite.comp.wgsl` | Active production for `RendererMode::HybridEffects`; production-dispatched split RT shaders now live in the hybrid shader tree while helper extraction continues incrementally. |
| Full raytracing and cinematic/pathtrace modes | `RendererMode::FullRaytracing` selects raytraced primary visibility with real-time caps and denoise; `PathTracePreview`/`CinematicPathTrace` keep path-traced primary visibility. | `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` | Active production monolithic RT primary shader. |
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

Production-dispatched split RT effect shaders have been promoted to `assets/shaders/wgpu/hybrid/`. The experimental directory keeps only the older raster/raytrace prototypes; new production wiring should use the hybrid tree.

## Duplication policy

`pathtrace.comp.wgsl` remains the single production monolithic RT shader. Split hybrid-effect passes must not fork full BVH/material/shadow traversal code; shared WGSL generation/fragments should be used for scene structs, `Params`, materials, BVH traversal, shadows, GI, clouds, and atmosphere as the split passes grow beyond screen-space/probe approximations.

## Production GI mode matrix

The GI selector is intentionally routed like an Unreal-style production renderer: raster modes keep a cheap baseline indirect solution, hybrid mode can add bounded ray-traced effects, full path-traced GI is reserved for path-traced primary visibility, and `FullRaytracing` uses bounded real-time RTGI/probe fallback instead of progressive path-traced GI.

| GI mode constant | RasterGame behavior | HybridEffects behavior | Path-traced primary behavior | Profile clamps |
| --- | --- | --- | --- | --- |
| `GI_MODE_OFF` | No indirect diffuse contribution beyond direct/raster lighting. | Same; RTGI pass is skipped. | Path-traced GI is disabled by quality/off state. | All profiles may force this when GI quality is off. |
| `GI_MODE_BAKED_LIGHTMAP` | Uses authored baked lightmap data as the static baseline raster GI. | Uses the same baked baseline and skips RTGI. | Promoted to path-traced GI when the renderer mode uses path-traced primary visibility. | Remains low-cost for `Indoor60FPS` and `Low`. |
| `GI_MODE_LIGHT_PROBES` | Uses authored/interpolated light probes as the static baseline raster GI. | Uses the same probe baseline and skips RTGI. | Promoted to path-traced GI when the renderer mode uses path-traced primary visibility. | `Indoor60FPS` forces this mode; `Low` falls back to this from RT/path GI requests. |
| `GI_MODE_SDFGI` | Dispatches the SDFGI cache/prepass/inject path for scalable dynamic GI when quality permits. | Dispatches the same SDFGI path unless the mode is explicitly RTGI one-bounce. | Promoted to path-traced GI when the renderer mode uses path-traced primary visibility. | `Low` caps quality before dispatch; `Cinematic` switches to path-traced GI. |
| `GI_MODE_RTGI_ONE_BOUNCE` | Clamped to light probes; RasterGame never dispatches RTGI. | Dispatches only the decomposed `rt_gi.comp.wgsl` one-bounce additive pass and feeds it to the hybrid compositor/history. | Promoted to path-traced GI; RTGI one-bounce is not used. | `Indoor60FPS` and `Low` clamp this away; `Cinematic` uses path-traced GI instead. |
| `GI_MODE_PATH_TRACED_PREVIEW` | Clamped to SDFGI because raster primary visibility does not path trace GI. | Clamped to SDFGI unless the user explicitly selects RTGI one-bounce. | Uses path-traced GI in path-traced primary modes. | `Low` clamps this to light probes; `Cinematic` forces path-traced primary visibility. |
