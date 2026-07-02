# Renderer parity guide

This document tracks renderer feature parity across the current WGPU raster,
hybrid, and path-traced paths. Use it as the first stop before adding a new
rendering implementation so new work extends the existing renderer instead of
forking shader logic.

## Status legend

- **Production**: wired from Rust, dispatched in normal renderer modes, and part
  of the supported renderer contract.
- **Partial**: wired or implemented for a subset of modes/features, but missing
  full production behavior or parity with the path-traced path.
- **Experimental**: prototype/reference shader code exists, but it is not the
  stable renderer contract or still lives in the experimental tree.
- **Not implemented**: no current renderer implementation found.

## Parity matrix

| Feature | Current status | Current implementation anchors | Parity / implementation notes |
| --- | --- | --- | --- |
| Raster primary visibility | **Production** | `src/rendering/wgpu_renderer/renderer_impl.inc.rs` creates and dispatches `primitive_gbuffer_pipeline`; active raster/PBR contract is documented in `SHADER_ARCHITECTURE.md`. | Raster primary visibility is production for raster and hybrid modes. Keep future raster work aligned with the existing primitive/PBR G-buffer path rather than the experimental raster prototype. |
| G-buffer / deferred shading | **Partial** | `renderer_impl.inc.rs` writes `primitive_gbuffer` and `pbr_gbuffer`; `assets/shaders/wgpu/experimental/hybrid_effects/raster.vert.wgsl` and `raster.frag.wgsl` are deferred/G-buffer prototypes. | G-buffer data exists and feeds hybrid effects, but a complete deferred lighting architecture is not the primary production contract yet. |
| Shadow maps / cascades | **Partial** | `renderer_impl.inc.rs` owns `raster_shadow_map`, `raster_shadow_view_proj`, primitive/PBR shadow passes, and reports `raster_shadow_maps_active`. | Single raster shadow-map support is wired. Cascaded shadow maps were not found in the current WGPU renderer, so requested `CascadedShadowMap` is now reported as active `RasterShadowMap` until a real cascade atlas/matrix-array path lands. |
| Virtual shadow maps | **Not implemented** | No VSM implementation is currently documented or wired in `renderer_impl.inc.rs`. | If planned, document the resource/page-table design here before adding shaders. Do not confuse future VSM work with the existing raster shadow map. |
| RT shadows | **Partial** | `renderer_impl.inc.rs` builds and dispatches `hybrid_rt_shadow_pipeline`; shader: `assets/shaders/wgpu/hybrid/rt_shadows.comp.wgsl`; monolithic path-traced shadow logic remains in `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl`. | Hybrid RT shadows are wired, and the split shader now lives under the production hybrid shader tree; shared traversal/material helper extraction is still incremental. |
| RT reflections | **Partial** | `renderer_impl.inc.rs` builds and dispatches `hybrid_rt_reflection_pipeline`; shader: `assets/shaders/wgpu/hybrid/rt_reflections.comp.wgsl`; compositor: `assets/shaders/wgpu/hybrid/hybrid_effects_composite.comp.wgsl`. | Hybrid RT reflections are wired as a bounded effect pass with history, while path-traced reflections remain in monolithic path tracing. |
| RTGI | **Partial** | `renderer_impl.inc.rs` builds and dispatches `hybrid_rt_gi_pipeline`; shader: `assets/shaders/wgpu/hybrid/rt_gi.comp.wgsl`; GI routing is summarized in `SHADER_ARCHITECTURE.md`. | Hybrid RTGI is a one-bounce additive mode. Raster modes use cheaper GI paths, while cinematic/path-traced modes promote to monolithic path-traced GI. |
| Full raytracing primary visibility | **Partial** | `RendererMode::FullRaytracing` selects `PrimaryVisibilityMethod::Raytraced` and uses the monolithic RT primary pipeline with denoise rather than the progressive path-traced mode. | Full RT is selectable and reported separately from path tracing, but still shares the monolithic RT shader until dedicated real-time shader entry points/helper extraction land. |
| Path tracing | **Production** | `renderer_impl.inc.rs` includes `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` and exposes `pathtrace_primary_active`; `SHADER_ARCHITECTURE.md` identifies it as the single production monolithic RT shader. | Path tracing is production and monolithic. New split passes must not fork the full pathtrace shader; extract shared WGSL helpers instead. |
| Denoising | **Production** | `renderer_impl.inc.rs` creates/dispatches `rt_denoise_pipeline` and `denoise_pipeline`; shaders: `assets/shaders/wgpu/hybrid/rt_denoise.comp.wgsl` and `assets/shaders/wgpu/hybrid/denoise.comp.wgsl`. | Both raytrace-specific and generic post denoise passes are production. Keep denoising changes centralized in these active shaders. |
| Temporal accumulation | **Partial** | `renderer_impl.inc.rs` maintains history textures/copies and hybrid reflection history; `assets/shaders/wgpu/hybrid/hybrid_effects_composite.comp.wgsl` blends GI history with `temporal_blend`; `assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl` is the active hybrid composition fallback. | Temporal history exists for denoise, hybrid effects, clouds, and post-FX, but parity differs by feature and mode. Document new histories and resolve ownership before adding more accumulation buffers. |
| Atmosphere / clouds | **Partial** | `renderer_impl.inc.rs` dispatches atmosphere LUTs and cloud directional shadow work; `SHADER_ARCHITECTURE.md` notes atmosphere LUTs as production support shaders and `cloud_raymarch.comp.wgsl` as experimental/future. | Atmosphere LUT support is production. Cloud/fog/atmosphere parity is split between LUTs, cloud shadow support, and monolithic pathtrace behavior; a standalone production cloud raymarch pass is not currently wired. |
| Transparency / translucency | **Partial** | `renderer_impl.inc.rs` builds/dispatches `hybrid_rt_transparency_pipeline`; shader: `assets/shaders/wgpu/hybrid/rt_transparency.comp.wgsl`; compositor: `assets/shaders/wgpu/hybrid/hybrid_effects_composite.comp.wgsl`. | Hybrid transparency is a split effect pass, not a complete unified translucency model across raster, hybrid, and path-traced paths. |

## Do not duplicate

- **Do not duplicate monolithic pathtrace logic.** `assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl` is the production monolithic path-tracing source for BVH traversal, material evaluation, direct/indirect lighting, shadows, GI, clouds, and atmosphere. When split hybrid passes need the same behavior, extract shared WGSL fragments/helpers instead of copying large sections.
- **Do not treat every experimental shader as a new production path.** Production split RT effect shaders live in `assets/shaders/wgpu/hybrid/`. Keep `experimental/hybrid_effects/raster.vert.wgsl`, `raster.frag.wgsl`, and `raytrace.comp.wgsl` as references unless Rust wiring and docs are updated together.
- **Do not add a second hybrid compositor without a migration plan.** `assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl` is still an active hybrid composition fallback, while `assets/shaders/wgpu/hybrid/hybrid_effects_composite.comp.wgsl` composes decomposed hybrid effect targets. New composition features should state which compositor owns them and when duplication will be removed.
- **Update this matrix with every renderer feature change.** If a feature moves from experimental to production, update this document, `SHADER_ARCHITECTURE.md`, Rust pipeline wiring, and shader comments in the same change.

## Renderer policy layer

`RendererMode` remains the public compatibility switch, but each frame now derives a `RendererPolicy` before dispatch. The policy maps the mode, `RendererProfile`, RT toggles, GI mode, material fallback tags, adaptive quality, and available pipelines into explicit methods for primary visibility, shadows, reflections, AO, GI, and transparency.

Current policy intent:

- `RasterGame`: raster primary visibility, raster shadow maps/cascades, SSAO/GTAO, SSR/probes, baked/probe/SDFGI GI, and raster/weighted transparency.
- `HybridEffects`: raster primary visibility with policy-selected RT contact shadows, RT reflection fallback, RTGI one-bounce, and RT transparency only when requested, supported, material-relevant, and within quality budget.
- `FullRaytracing`: real-time raytraced primary visibility with RT shadows/reflections, bounded RTGI/probe fallback, RT or screen-space transparency, denoise, and fallback reasons independent from path tracing.
- `PathTracePreview` / `CinematicPathTrace`: path-traced primary visibility with path-traced GI/reflections/transparency and ray-traced shadows when the cinematic pipeline is available; otherwise the renderer reports the selected policy separately from actual active pipeline status.

`RendererHybridFeatureStatus` now carries both legacy booleans and the active policy methods, so profiler/HUD consumers can distinguish “hybrid RT reflections active” from a cheaper policy such as SSR, probes, or SSR with RT fallback.
