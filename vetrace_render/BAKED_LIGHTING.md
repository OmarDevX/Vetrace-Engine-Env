# Baked lightmaps and light probes

`vetrace_render` provides an explicit CPU bake for static diffuse lighting and a load-only runtime path.

## API

```rust
use vetrace_render::{
    bake_and_save_baked_lighting, load_baked_lighting,
    set_baked_lighting_runtime_mode,
    BakedLightingBakeConfig, BakedLightingRuntimeMode,
    BakedLightmapReceiver, BakedLightProbeReceiver,
};

// Static geometry: receives a lightmap and participates in bake occlusion/bounce.
static_actor.insert(engine, BakedLightmapReceiver::default())?;

// Moving geometry: samples the baked directional probe volume.
player.insert(engine, BakedLightProbeReceiver::default())?;

// Explicit tool/runtime bake. This is the only API that starts a bake.
let report = bake_and_save_baked_lighting(
    engine,
    "assets/baked_lighting/level.vlight",
    &BakedLightingBakeConfig::default(),
)?;

// Normal runtime. This never bakes or falls back to a bake.
load_baked_lighting(engine, "assets/baked_lighting/level.vlight")?;

// Optional high-quality composition: baked indirect GI + realtime direct shadows.
set_baked_lighting_runtime_mode(
    engine,
    BakedLightingRuntimeMode::HybridRealtimeDirect,
);
```

The version-5 `.vlight` file is a versioned binary containing a two-layer RGBA16F atlas, stable object-to-atlas mappings, L2 spherical-harmonic irradiance probes, and baked directional visibility data for leak-resistant interpolation. The top atlas half stores combined direct + indirect lighting for the cheapest runtime mode. The bottom half stores indirect-only lighting so a high-quality profile can keep the old realtime direct-light shadow path without double-lighting the scene.

Version 5 replaced the old RGBM8 atlas because its shared 8-bit multiplier produced visible contour bands in smooth GI gradients. Existing version-4 `.vlight` files must be rebaked.

Older version-1 files are rejected deliberately and must be rebaked.

## Geometry requirements

Built-in cube, plane/quad, sphere, and capsule primitives receive generated non-overlapping lightmap UVs. Imported meshes use glTF `TEXCOORD_1` as UV2. A mesh without valid UV2 still participates as an occluder/bounce surface when marked as a receiver, but it is skipped as a lightmap receiver. Bounce coloration currently uses each material's `base_color`; base-color textures are still applied at runtime, but they do not tint the baked indirect bounce.

Rebake after changing static geometry, receiver transforms, static materials, baked lights, texel density, or bake filtering. Stable object keys prevent a changed receiver from silently using another object's atlas region, but the file is not a substitute for scene-version management.

## Runtime modes and cost

`BakedLightingRuntimeMode::BakedOnly` samples the combined atlas half and suppresses duplicate realtime directional/ambient lighting on static receivers. Static receivers use one linear atlas sample. Moving receivers use trilinearly interpolated directional probes uploaded in the object uniform.

`BakedLightingRuntimeMode::HybridRealtimeDirect` still uses only one atlas sample, but selects the indirect-only half and keeps realtime direct lights and their shadow maps. This gives smooth dynamic/high-resolution shadows plus baked GI, at the expected realtime shadow-map cost.

Normal gameplay performs no CPU baking, ray tracing, or probe-grid construction. Neither mode is literally zero GPU work.

## Lightmap quality

`lightmap_resolution` is the minimum per-receiver tile size. `lightmap_texels_per_unit` automatically increases large receiver tiles up to 512 texels. The renderer default is `8.0` texels per world unit.

`lightmap_filter_radius` performs a coverage-aware full-float filter during the CPU bake before the atlas is converted once to RGBA16F. It smooths stair-stepped shadow edges without sampling across different UV charts. Zero disables it; practical values are 1 for balanced and 2 for high quality.

Increasing density raises atlas memory and file size. Increasing the filter radius raises bake time but adds no runtime texture samples.

## Indirect bounce quality

`indirect_bounces` controls the number of iterative diffuse probe solves. A value of `1` preserves the original single-bounce result. Values from `2` to `4` progressively fill enclosed spaces and make wall-to-object color bleeding more visible. Each additional bounce increases CPU bake time approximately linearly but has no runtime sampling cost.

`indirect_bounce_decay` controls how much energy survives each extra bounce. Keep it below `1.0`; practical values are `0.55..0.75`. `indirect_intensity` is a final artistic multiplier for the indirect lightmap layer and does not affect direct baked lighting. Dynamic objects have their own `BakedLightProbeReceiver::intensity` multiplier.

## Simple Shooter

Balanced bake:

```text
cargo run -p simple_shooter -- --balanced --bake-lighting --no-main-menu
```

High-quality bake:

```text
cargo run -p simple_shooter -- --high-quality --bake-lighting --no-main-menu
```

Normal gameplay only loads the saved result. To force the matching hybrid profile from the CLI:

```text
cargo run -p simple_shooter -- --high-quality --no-main-menu
```

Without an explicit profile flag, gameplay uses the graphics profile saved by the in-game settings UI.

Simple Shooter presets are:

- Low Spec: 4 texels/unit, no bake filter, 108 probes.
- Balanced: 8 texels/unit, radius-1 filter, 256 probes, baked-only static lighting.
- High Quality: 12 texels/unit, radius-2 filter, 500 probes, baked indirect GI plus the original realtime soft shadow path.

Files are stored under `simple_shooter/assets/baked_lighting/` as `lobby.vlight` and `game_<index>.vlight`.

## Debugging

The renderer exposes four runtime debug modes through `cycle_baked_lighting_debug_mode`:

- `Off`: normal shaded result.
- `Lightmap`: decoded combined direct + indirect atlas irradiance only, even while hybrid runtime composition is active. This makes baked static shadows directly inspectable.
- `LightmapUv`: UV2 chart coordinates.
- `Probes`: probe irradiance on receivers and a visible translucent sphere at every probe-grid position. Each sphere samples the actual directional probe volume at its own position and is excluded from the shadow pass.

Simple Shooter binds this cycle to `B` and prints the active mode and marker count. With the default balanced grid, Probes mode should report and display 256 markers; a high-quality bake should show 500.

Baked-only shadows represent geometry and lights that were static at bake time. A moving player or bot cannot cast a future shadow into a precomputed lightmap. High Quality mode solves that by combining the indirect-only bake with the normal realtime shadow pass.

## Cornell Box validation example

A self-contained Cornell Box example is included to validate the complete baked-lighting path:

```bash
# Explicit CPU bake. This writes/overwrites cornell_box.vlight.
cargo run -p vetrace_render --example cornell_box_baked_gi --features wgpu_window -- --bake-lighting

# Normal load-only runtime. No bake occurs.
cargo run -p vetrace_render --example cornell_box_baked_gi --features wgpu_window
```

The scene contains red and green diffuse walls, two static white boxes, a finite baked rectangular ceiling emitter, and a moving white sphere using `BakedLightProbeReceiver`. It is designed to resemble a conventional Cornell Box: broad warm illumination, soft penumbrae, readable shadowed faces, controlled red/green color bleeding, and smooth probe interpolation. Its default preset uses five diffuse bounces, `0.70` bounce decay, `1.22` indirect-lightmap intensity, `1.18` probe-receiver intensity, 49 area-light samples, 384 probe rays, and plausible Cornell material albedos.

The bake can be tuned directly from the command line:

```bash
cargo run -p vetrace_render --example cornell_box_baked_gi --features wgpu_window -- \
  --bake-lighting \
  --area-light-intensity 44 \
  --area-light-samples 25 \
  --indirect-bounces 4 \
  --bounce-decay 0.68 \
  --indirect-intensity 1.12 \
  --lightmap-intensity 1.0 \
  --probe-intensity 1.08
```

These options affect baking except `--probe-intensity`, which controls the moving sphere's runtime probe response. Re-run with `--bake-lighting` after changing the rectangular emitter, bounce count, decay, indirect intensity, or lightmap intensity.

Controls:

- `B`: cycles `Off -> Lightmap -> LightmapUv -> Probes -> Off`.
- `M`: toggles `BakedOnly` and `HybridRealtimeDirect`.
- `Space`: pauses/resumes the moving probe-test sphere.
- `Escape`: exits.

`BakedOnly` is the normal Cornell render and proves that the rectangular emitter's direct illumination, soft shadows, and indirect color bleed are present in the `.vlight` file. `HybridRealtimeDirect` intentionally shows the indirect-only atlas/probe layer; because `BakedRectAreaLight` has no realtime evaluation path, Hybrid is a diagnostic rather than the intended final appearance for this example.

## Baked rectangular area lights

Attach `BakedRectAreaLight` to a rectangle-shaped emitter that lies in the
entity's local XZ plane. The emitter faces local `+Y`; rotate the entity to aim
it. The CPU baker samples the finite surface directly, producing broad Cornell-
box illumination and soft penumbrae without adding a runtime light loop.

```rust
.with(BakedRectAreaLight {
    color: Vec3::new(1.0, 0.90, 0.76),
    intensity: 44.0,
    width: 1.35,
    height: 0.92,
    samples: 25,
    two_sided: false,
    enabled: true,
})
```

The visible `Material::emissive` value is intentionally independent from the
bake intensity. When both components are on the same entity, the baker suppresses
the material emission and uses the explicitly sampled rectangle, avoiding double
counting. Increasing `samples` smooths penumbrae but raises bake time only; it
has no normal runtime cost.

Baked-lighting file version 5 stores the RGBA16F lightmap atlas plus combined and indirect-only L2 SH probe grids with directional visibility distances. Older `.vlight` files must be rebaked.
`BakedOnly` uses combined direct + indirect probes, while
`HybridRealtimeDirect` uses indirect-only probes to avoid counting realtime
lights twice. All pre-version-5 `.vlight` files must be rebaked.

## L2 probe visibility and tone mapping

Dynamic receivers evaluate nine RGB L2 spherical-harmonic coefficients. The coefficients are pre-convolved for Lambertian diffuse response during baking, so runtime work is a small nine-term dot product. Each probe also stores free-space distances along the six principal directions. Trilinear interpolation down-weights probe corners that are separated from the receiver by nearby geometry, reducing red/green leakage through walls without runtime ray tracing.

`PostProcessing` now controls exposure, display gamma adjustment, and the tone mapper (`Off`, `Aces`, `Neutral`, or `Reinhard`). The Cornell Box example binds `J`/`K` to exposure and `T` to tone-mapper cycling. The WGPU surface is sRGB, so gamma `2.2` is neutral and does not double-encode the image.
