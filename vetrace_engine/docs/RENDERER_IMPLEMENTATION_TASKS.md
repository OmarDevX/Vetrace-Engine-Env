# Renderer feature completion tasks

This document tracks the remaining work needed to turn the current renderer into
an Unreal-style selectable pipeline stack:

- **RasterGame**: raster primary visibility with cheap raster/screen-space
  solutions.
- **HybridEffects**: raster primary visibility plus selectively enabled RT
  effects when requested, material-relevant, supported, and within budget.
- **Full raytracing**: raytraced primary visibility for real-time use, distinct
  from progressive/path-traced preview.
- **PathTracePreview / CinematicPathTrace**: path-traced primary visibility for
  preview, validation, screenshots, and cinematic output.

The task list intentionally combines short follow-ups into larger work packages
so changes land as coherent renderer milestones instead of isolated one-off
branches. Before implementing any task below, check `RENDERER_PARITY.md`,
`SHADER_ARCHITECTURE.md`, and `EFFECT_FALLBACKS.md` to avoid duplicating shader
logic or creating another compositor path.

## Current remaining gap summary

| Area | Current state | Completion target |
| --- | --- | --- |
| Cascaded / scalable raster shadows | Single raster shadow-map support is wired and `ShadowMethod::CascadedShadowMap` exists, but no true cascade atlas/splits/matrix array path is documented. | Implement CSM as a real raster shadow method or downgrade policy/status to single-map until CSM exists. |
| Full raytracing primary visibility | `PrimaryVisibilityMethod::Raytraced` exists, but `RendererMode` has no full-raytracing variant and policy currently routes non-raster primary visibility to path tracing. | Add a real-time full-raytracing mode/preset that is separate from path tracing and hybrid raster-primary. |
| Hybrid RT shader production ownership | Hybrid RT shadows/reflections/GI/transparency/RTAO are wired, but several production-used shaders still live under `experimental/hybrid_effects`. | Extract shared WGSL helpers and move production split shaders into `assets/shaders/wgpu/hybrid/`. |
| Deferred/raster feature contract | G-buffer, PBR compose, SSR, AO, raster shadows, and GI resolve are present, but the authoritative channel/history/ownership contract is spread across code and docs. | Document and enforce one G-buffer/resolve/history contract for raster and hybrid passes. |
| GI and lighting parity | SDFGI, GI resolve, RTGI, lightmap/probe fallbacks, and path-traced GI exist as separate paths with differing quality/ownership. | Finish one GI contract that cleanly maps baked/probe/SDFGI/RTGI/path-traced GI into policy and debug views. |
| Transparency parity | Hybrid RT transparency is wired, while raster alpha/weighted/screen-space transparency and path-traced transparency are policy-selected but not unified. | Finish a unified transparency ladder and compositor contract across raster, hybrid, full RT, and path tracing. |
| Debug/profiling truth | Requested/active status exists, but new methods such as future CSM/full RT need truthful pass-level activation, fallback reasons, and timings. | Keep HUD/profiler/debug overlays accurate for every requested vs active method. |

## Task 1: Finish scalable raster shadowing and policy truth

### Goal

Make raster shadows a complete cheap baseline for `RasterGame` and the fallback
layer for `HybridEffects`.

### Current anchors

- `ShadowMethod::{RasterShadowMap, CascadedShadowMap, Raytraced, RasterPlusRtContact}` exists in `renderer.rs`.
- The current WGPU renderer owns a single `raster_shadow_map` and
  `raster_shadow_view_proj` path.
- `RENDERER_PARITY.md` still marks shadow maps/cascades as partial because true
  cascades were not found.

### Required implementation

1. Decide whether the next milestone is **single-map only** or **true CSM**:
   - If single-map only, stop reporting `CascadedShadowMap` as active when only
     the single map is used.
   - If CSM, add a directional shadow atlas or array texture, cascade split
     data, and a cascade matrix array uniform/storage buffer.
2. Extend the raster shadow pass to render primitives and PBR meshes into all
   active cascades.
3. Update `hybrid_compose.comp.wgsl` to select the correct cascade by view/world
   depth and sample the atlas/array.
4. Keep `RasterPlusRtContact` layered as: CSM/single raster shadow first, RT
   contact/hero shadow second.
5. Update `RendererHybridFeatureStatus` so requested/active shadow method is
   truthful:
   - requested `CascadedShadowMap`, active `RasterShadowMap` if only one map is
     available;
   - requested `RasterPlusRtContact`, active `RasterShadowMap` if RT pipeline or
     BVH data is unavailable.
6. Add/update debug view for raster shadow cascade selection and shadow factor.
7. Update `RENDERER_PARITY.md` from partial to production only after CSM or the
   final chosen raster-shadow contract is complete.

### Suggested files

- `vetrace_engine/src/rendering/renderer.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/renderer.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/raster_shadow.wgsl`
- `vetrace_engine/docs/RENDERER_PARITY.md`

### Acceptance criteria

- `ShadowMethod::CascadedShadowMap` only reports active when real cascades are
  used.
- Raster and hybrid modes have stable shadow output without requiring RT.
- Hybrid mode can layer RT contact shadows over raster shadows without replacing
  the cheap baseline.
- Profiler/HUD reports requested and active shadow methods separately.

## Task 2: Add a real full-raytracing mode distinct from path tracing

### Goal

Add a real-time raytraced primary visibility mode that is neither hybrid
raster-primary nor progressive path tracing.

### Current anchors

- `PrimaryVisibilityMethod::Raytraced` exists.
- `RendererMode` currently has `RasterGame`, `HybridEffects`,
  `PathTracePreview`, and `CinematicPathTrace` only.
- `RendererPolicy::derive` currently maps `PathTracePreview` and
  `CinematicPathTrace` to `PathTraced`, not `Raytraced`.

### Required implementation

1. Add a public mode or preset for full raytracing:
   - Preferred public enum option: `RendererMode::FullRaytracing = 4`.
   - Alternative compatibility option: add a preset/config field that maps to
     `PrimaryVisibilityMethod::Raytraced` without changing serialized enum
     values.
2. Route full raytracing through `RendererPolicy::derive`:
   - `primary_visibility = Raytraced`;
   - shadows/reflections/GI/transparency default to real-time RT methods;
   - bounces/samples are clamped for frame-rate, unlike cinematic path tracing.
3. Add a raytraced-primary compute path:
   - either a constrained entry point in the current pathtrace shader;
   - or a new shader that imports shared BVH/material/lighting helpers.
4. Keep full raytracing denoised and budgeted:
   - low sample counts;
   - adaptive quality caps;
   - RT denoise/history;
   - no long progressive accumulation requirement.
5. Make path tracing remain the high-quality/progressive mode:
   - `PathTracePreview` and `CinematicPathTrace` keep path-traced primary
     visibility;
   - full raytracing should not silently become path tracing.
6. Update profiler/debug UI:
   - requested primary `Raytraced`, active primary `Raytraced` when ready;
   - fallback to raster or path-traced primary must include a fallback reason.
7. Update docs:
   - `RENDERER_PARITY.md`;
   - `SHADER_ARCHITECTURE.md`;
   - any user-facing renderer preset/config docs.

### Suggested files

- `vetrace_engine/src/rendering/renderer.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl`
- shared WGSL helpers under `vetrace_engine/assets/shaders/wgpu/hybrid/`
- `vetrace_engine/docs/RENDERER_PARITY.md`
- `vetrace_engine/docs/SHADER_ARCHITECTURE.md`

### Acceptance criteria

- Engine users can choose a real-time full-raytracing mode separate from path
  tracing.
- `PrimaryVisibilityMethod::Raytraced` is actually selected and dispatched.
- Full raytracing can be disabled/fallbacked independently from path tracing.
- HUD/profiler distinguishes raster, hybrid, full RT, and path-traced primary
  visibility.

## Task 3: Promote production hybrid RT shaders out of experimental

### Goal

Make shader ownership match runtime ownership: production-dispatched shaders
should not permanently live in the experimental tree.

### Current anchors

The WGPU renderer dispatches split hybrid effect pipelines for shadows,
reflections, GI, transparency, and RTAO, while several of those WGSL files still
live under `assets/shaders/wgpu/experimental/hybrid_effects/`.

### Required implementation

1. Extract or finalize shared WGSL helpers for:
   - `Params` / shader ABI structs;
   - G-buffer decode;
   - material decode/evaluation;
   - PBR lighting;
   - BVH / triangle BVH traversal;
   - shadow ray helpers;
   - reflection ray helpers;
   - GI ray helpers;
   - transparent/refraction helpers.
2. Replace ad-hoc copied code in split passes with shared includes/concats or a
   small shader-generation path.
3. Move production split shaders into `assets/shaders/wgpu/hybrid/`:
   - `rt_shadows.comp.wgsl`;
   - `rt_reflections.comp.wgsl`;
   - `rt_gi.comp.wgsl`;
   - `rt_transparency.comp.wgsl`;
   - `rt_ao.comp.wgsl` if kept as production RTAO;
   - `hybrid_effects_composite.comp.wgsl` or merge into the active compositor.
4. Leave only true prototypes in `experimental/hybrid_effects/`.
5. Update all `include_str!` paths in Rust.
6. Update shader comments and docs so production/experimental status is clear.
7. Run WGSL layout/syntax validation after moving paths.

### Suggested files

- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/assets/shaders/wgpu/hybrid/*`
- `vetrace_engine/assets/shaders/wgpu/experimental/hybrid_effects/*`
- `vetrace_engine/docs/SHADER_ARCHITECTURE.md`
- `vetrace_engine/docs/RENDERER_PARITY.md`
- `scripts/validate_wgsl_layouts.py`
- `scripts/validate_wgsl_syntax.py`

### Acceptance criteria

- No shader dispatched in normal renderer modes is mislabeled as experimental
  unless the docs explicitly mark it as temporary.
- Split passes share traversal/material/PBR helper code with the monolithic path
  where practical.
- No duplicate monolithic pathtrace shader is introduced.
- `RENDERER_PARITY.md` and `SHADER_ARCHITECTURE.md` agree on production shader
  ownership.

## Task 4: Finish the raster/deferred feature contract

### Goal

Make G-buffer, AO, SSR, raster shadows, GI resolve, and hybrid composition use a
single documented contract.

### Current anchors

- Primitive and PBR mesh raster passes write G-buffer data.
- `hybrid_compose.comp.wgsl` consumes G-buffer data, raster shadows, GI, AO/SSR
  inputs, and PBR helpers.
- SSR and AO now have dedicated textures/passes.
- GI resolve exists, but quality and ownership differ by GI method.

### Required implementation

1. Document the exact G-buffer layout:
   - `gbuf_albedo` channels and alpha/object-valid meaning;
   - `gbuf_normal` encoding;
   - `gbuf_material` packed channels;
   - depth texture conventions;
   - motion/history ownership if applicable.
2. Ensure primitive and mesh raster paths encode the same material fields.
3. Make each post-raster feature declare ownership of its output/history:
   - AO output/history;
   - SSR color/history;
   - RT reflection output/history;
   - GI output/history;
   - transparency output/history if needed.
4. Make `hybrid_compose.comp.wgsl` consume stable feature inputs rather than
   reaching into method-specific details.
5. Add debug views for:
   - albedo;
   - normals;
   - material roughness/metallic/emissive;
   - depth;
   - AO;
   - SSR confidence;
   - raster shadow factor;
   - GI buffer;
   - final composite feature contribution.
6. Add comments in the shader files that write/read the contract.
7. Update validation scripts if a static layout check can catch drift.

### Suggested files

- `vetrace_engine/assets/shaders/wgpu/hybrid/primitive_gbuffer.wgsl`
- `vetrace_engine/shaders/simple_pbr.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/ssr.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/ambient_occlusion.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl`
- `vetrace_engine/docs/RENDERER_PARITY.md`

### Acceptance criteria

- Primitive and mesh raster objects produce compatible G-buffer data.
- Compositor inputs are method-agnostic where possible.
- Debug views reveal whether visual bugs come from G-buffer, AO, SSR, GI,
  shadow, or composite stages.
- No pass silently overwrites another pass's history buffer.

## Task 5: Complete GI parity across baked, probes, SDFGI, RTGI, and path tracing

### Goal

Make all GI methods resolve through one understandable policy and buffer model,
while preserving path tracing as the high-quality ground-truth path.

### Current anchors

- GI policy methods exist.
- Raster modes can use baked/probe/SDFGI.
- Hybrid can use RTGI one-bounce.
- Path-traced modes promote to path-traced GI.
- `RENDERER_PARITY.md` still marks RTGI and temporal accumulation as partial.

### Required implementation

1. Finalize `gi_buffer` ownership:
   - `Off` writes black/neutral indirect;
   - `BakedLightmap` resolves authored lightmap data;
   - `LightProbes` resolves probe/ambient data;
   - `SDFGI` resolves SDFGI radiance;
   - `RTGIOneBounce` resolves RTGI output and history;
   - `PathTraced` bypasses raster GI buffer and uses path-traced radiance.
2. Ensure the compositor only needs the resolved GI buffer in raster/hybrid
   primary modes.
3. Add or finish GI debug views for each method and final resolved GI.
4. Make profile clamping visible:
   - requested `RTGIOneBounce`, active `LightProbes` under `Low` or
     `Indoor60FPS`;
   - requested `PathTraced`, active fallback if path tracing unavailable.
5. Ensure RTGI does not duplicate pathtrace GI logic; share traversal/material
   helpers where practical.
6. Define temporal ownership:
   - which pass owns RTGI history;
   - which pass owns SDFGI cache invalidation;
   - when history resets on camera/scene changes.
7. Update docs after the final GI contract is stable.

### Suggested files

- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/types.rs`
- `vetrace_engine/assets/shaders/wgpu/hybrid/gi_resolve.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/sdfgi_*.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/pathtrace.comp.wgsl`
- `vetrace_engine/docs/SHADER_ARCHITECTURE.md`
- `vetrace_engine/docs/RENDERER_PARITY.md`

### Acceptance criteria

- Requested/active GI method is always truthful.
- Raster, hybrid, full RT, and path-traced GI choices are clear in the HUD.
- Final compose sees one resolved GI contract in raster/hybrid modes.
- Path-traced GI remains separate and does not require raster GI buffers.

## Task 6: Unify transparency and translucency across renderer modes

### Goal

Finish the transparency ladder so raster, hybrid, full raytracing, and path
tracing all have clear behavior.

### Current anchors

- `TransparencyMethod::{RasterAlpha, WeightedOIT, ScreenSpaceRefraction,
  Raytraced, PathTraced}` exists.
- Hybrid RT transparency is wired through a split effect pipeline.
- `RENDERER_PARITY.md` still marks transparency/translucency as partial.

### Required implementation

1. Define material properties/tags that select transparency behavior:
   - ordinary alpha blend;
   - weighted OIT;
   - screen-space refraction;
   - RT refraction/transparency;
   - path-traced transparency.
2. Implement or verify the cheap raster transparency path:
   - sorted alpha or weighted blended OIT;
   - predictable interaction with depth/G-buffer;
   - no accidental RT requirement.
3. Implement or verify screen-space refraction fallback for expensive transparent
   materials when RT is unavailable or too costly.
4. Make hybrid RT transparency run only when requested and material-relevant.
5. Add full raytracing and path-traced transparency behavior after full RT mode
   lands.
6. Update compositor ownership:
   - transparent radiance input;
   - opacity/transmittance convention;
   - history ownership if temporal filtering is used.
7. Expose requested/active transparency method in debug/HUD with fallback reason.

### Suggested files

- `vetrace_engine/src/rendering/renderer.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/assets/shaders/wgpu/experimental/hybrid_effects/rt_transparency.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/experimental/hybrid_effects/composite.comp.wgsl`
- `vetrace_engine/assets/shaders/wgpu/hybrid/hybrid_compose.comp.wgsl`
- material/component files that expose transparency flags or fallback tags
- `vetrace_engine/docs/EFFECT_FALLBACKS.md`
- `vetrace_engine/docs/RENDERER_PARITY.md`

### Acceptance criteria

- Raster-only materials never trigger RT transparency.
- Expensive transparent materials can use RT only in capable hybrid/full RT
  modes.
- Screen-space/raster fallback is visible in requested-vs-active status.
- Transparency docs match actual renderer behavior.

## Task 7: Keep profiler, debug overlays, and docs truthful

### Goal

Every renderer feature should expose what was requested, what actually ran, why
it fell back, and whether the reported timing is real.

### Current anchors

- `RendererHybridFeatureStatus` tracks requested/active methods and fallback
  reasons.
- The profiler HUD displays requested vs active primary/shadow/reflection/AO/GI
  and transparency methods.
- GPU timestamp query reporting exists for several passes.

### Required implementation

1. Extend status when new methods are added:
   - CSM active vs single-map fallback;
   - full RT primary requested/active;
   - weighted OIT/screen-space refraction requested/active;
   - production shader ownership/fallback state if safe shader mode disables a
     pass.
2. Add pass-specific timing where practical:
   - AO;
   - SSR;
   - raster shadow map/CSM separately from main raster G-buffer;
   - transparency;
   - GI resolve separate from RTGI producer.
3. Keep inactive pass timings at `0.0` instead of estimated values.
4. Add debug overlays for fallback decisions documented in `EFFECT_FALLBACKS.md`.
5. Update docs in the same change as behavior changes:
   - `RENDERER_PARITY.md` status row;
   - `SHADER_ARCHITECTURE.md` active shader map;
   - this task file, removing completed items.
6. Add validation/checklist notes for WGSL layout/syntax tests.

### Suggested files

- `vetrace_engine/src/rendering/renderer.rs`
- `vetrace_engine/src/rendering/wgpu_renderer/renderer_impl.inc.rs`
- `vetrace_engine/docs/EFFECT_FALLBACKS.md`
- `vetrace_engine/docs/RENDERER_PARITY.md`
- `vetrace_engine/docs/SHADER_ARCHITECTURE.md`
- `scripts/validate_wgsl_layouts.py`
- `scripts/validate_wgsl_syntax.py`

### Acceptance criteria

- HUD never implies a feature is active just because it was requested.
- Fallback reasons are actionable: missing pipeline, missing hardware, safe
  shader mode, missing BVH/acceleration data, missing lightmaps/probes, or
  profile-budget downgrade.
- Docs do not claim AO/SSR/RT features are missing after they are wired, and do
  not claim CSM/full RT are production until they actually are.

## Recommended implementation order

1. **Task 1: scalable raster shadows / truthful CSM status** because cheap
   raster and hybrid fallback quality depends on shadows.
2. **Task 4: raster/deferred feature contract** because it stabilizes G-buffer,
   AO, SSR, GI, and compositor inputs before more modes are added.
3. **Task 5: GI parity** because GI currently spans the most paths and history
   ownership rules.
4. **Task 6: transparency parity** because it needs the same policy/fallback
   model as reflections but affects sorting/composition.
5. **Task 2: full raytracing mode** once shared helpers and feature contracts are
   stable enough to avoid another monolithic fork.
6. **Task 3: production shader ownership cleanup** after shared helpers are
   ready and split shaders no longer duplicate pathtrace logic.
7. **Task 7: profiler/debug/docs truth pass** at the end of each milestone and
   again before marking parity rows production.
