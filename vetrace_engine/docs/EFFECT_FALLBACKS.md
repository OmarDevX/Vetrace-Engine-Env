# Expensive Effect Fallbacks

The hybrid renderer degrades costly effects by profile, material tags, distance, and size so performance mode does not collapse directly to black/no-effect output.

## Fallback ladders

- Reflections: ray traced reflection -> screen-space reflection -> reflection probe -> environment.
- Global illumination: RTGI -> SDFGI -> light probes -> baked lightmap -> ambient.
- Shadows: RT soft shadow -> raster shadow map -> contact shadow -> none.
- Transparency: RT refraction -> screen-space refraction -> alpha blend.
- Fog/cloud shadows: RT/object shadowing -> shadow map -> ambient-only lighting.

## Material tags

`PbrMaterial::fallback_tags` accepts `MATERIAL_TAG_*` bits:

- `MATERIAL_TAG_NEEDS_ACCURATE_REFLECTION`: hero/mirror materials can keep high-quality RT longer.
- `MATERIAL_TAG_CAN_USE_PROBE`: surfaces that tolerate probe/environment reflection fallback.
- `MATERIAL_TAG_RASTER_ONLY`: force raster/contact/alpha paths and skip RT participation.
- `MATERIAL_TAG_TRANSPARENT_EXPENSIVE`: glass/liquid that should be prioritized for RT refraction when budget allows.
- `MATERIAL_TAG_EMISSIVE_STATIC`: emissive surfaces suitable for baked/probe GI instead of dynamic RTGI.

## Threshold policy

`EffectFallbackPolicy::for_profile` defines profile-specific thresholds. Small or far objects skip RT shadow participation, rough materials use SSR/probes before RT reflections, and `Indoor60FPS` disables cloud/object shadowing while retaining probes/lightmaps for stable indoor lighting.

## Debug overlay

Enable `PostProcessing::fallback_policy.debug_overlay` or select ray debug view `6` to color fallback decisions: green for high-quality RT/hero pixels, cyan for SSR/probe/raster-only material fallback, amber for raster shadow fallback, and blue for ambient/lightmap-style fallback.

## Policy-driven feature selection

The renderer derives a single `RendererPolicy` once per frame instead of letting independent booleans decide passes in different parts of the render flow. The policy chooses concrete methods for:

- primary visibility: raster, ray-traced, or path-traced;
- shadows: off, raster map, cascaded map, ray-traced, or raster plus RT contact shadows;
- reflections: off, probes, SSR, SSR with RT fallback, ray-traced, or path-traced;
- ambient occlusion: off, SSAO, GTAO, or RTAO;
- GI: off, baked lightmap, probes, SDFGI, one-bounce RTGI, or path-traced;
- transparency: raster alpha, weighted OIT, screen-space refraction, ray-traced, or path-traced.

Material tags are inputs to the policy: raster-only materials suppress RT methods, probe-friendly materials prefer probe fallback under low budget, accurate-reflection tags enable SSR-then-RT fallback in hybrid mode, expensive transparent materials can opt into RT transparency when supported, and static emissive materials prefer baked/probe GI.
