// Production-active decomposed hybrid effects compositor.
// Binding 0 is the lit raster color_texture, not raw G-buffer albedo.
@group(0) @binding(0) var lit_raster_tex: texture_2d<f32>;
@group(0) @binding(1) var out_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(43) var resolved_gi_buffer: texture_2d<f32>;
@group(0) @binding(5) var rt_shadow_mask: texture_2d<f32>;
@group(0) @binding(46) var rt_reflection_radiance: texture_2d<f32>;
@group(0) @binding(7) var rt_gi_radiance: texture_2d<f32>;
@group(0) @binding(8) var rt_transparency_radiance: texture_2d<f32>;
@group(0) @binding(9) var atmosphere_overlay: texture_2d<f32>;
@group(0) @binding(10) var cloud_overlay: texture_2d<f32>;
@group(0) @binding(11) var cloud_transmittance: texture_2d<f32>;
@group(0) @binding(12) var gbuf_material: texture_2d<u32>;
@group(0) @binding(44) var ambient_occlusion_tex: texture_2d<f32>;
@group(0) @binding(45) var ssr_reflection_radiance: texture_2d<f32>;

struct CompositeParams {
    temporal_blend: f32,
    rt_gi_enabled: u32,
    rt_reflections_enabled: u32,
    ssr_enabled: u32,
    rt_shadows_enabled: u32,
    rt_transparency_enabled: u32,
    atmosphere_enabled: u32,
    clouds_enabled: u32,
    debug_view: u32,
};
@group(0) @binding(4) var<uniform> comp_params: CompositeParams;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(out_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    let raster_sample = textureLoad(lit_raster_tex, pixel, 0);
    // color alpha carries object id for selection outline; -1 means sky/miss.
    // Do not use alpha as a coverage selector here: hybrid_compose already wrote
    // the configured params.skycolor fallback into raster_sample.rgb for sky/miss
    // pixels. Replacing negative-alpha pixels with a hardcoded gradient breaks
    // the engine/editor sky color setting.
    let base = raster_sample.rgb;
    // GI resolve runs full resolution. Sampling only even pixels created visible
    // 2x2 block/pixel artifacts in SDFGI and RTGI debug/composite views.
    let gi_uv = pixel;
    // GI resolve owns method selection and RTGI temporal/spatial filtering; composite only consumes gi_buffer.
    let resolved_gi = textureLoad(resolved_gi_buffer, gi_uv, 0).rgb;
    let ao = clamp(textureLoad(ambient_occlusion_tex, pixel, 0).r, 0.0, 1.0);
    if (comp_params.debug_view == 12u) {
        let ssr_dbg = textureLoad(ssr_reflection_radiance, pixel, 0);
        let right_half = id.x >= dims.x / 2u;
        let bottom_half = id.y >= dims.y / 2u;
        if (!right_half && !bottom_half) {
            textureStore(out_tex, pixel, vec4<f32>(base, raster_sample.a));
        } else if (right_half && !bottom_half) {
            textureStore(out_tex, pixel, vec4<f32>(vec3<f32>(ao), raster_sample.a));
        } else if (!right_half && bottom_half) {
            textureStore(out_tex, pixel, vec4<f32>(max(ssr_dbg.rgb, vec3<f32>(0.0)), raster_sample.a));
        } else {
            textureStore(out_tex, pixel, vec4<f32>(max(resolved_gi, vec3<f32>(0.0)), raster_sample.a));
        }
        return;
    }
    let shadow = select(1.0, textureLoad(rt_shadow_mask, pixel, 0).r, comp_params.rt_shadows_enabled != 0u);
    let transparency = select(vec3<f32>(0.0), textureLoad(rt_transparency_radiance, pixel, 0).rgb, comp_params.rt_transparency_enabled != 0u);
    // The direct-lighting pass already applies AO, GI, SSR/probe/RT reflections and raster shadows.
    // Keep this final compositor from double-adding those terms; it should only layer optional
    // screen-space/RT contact shadows, transparency, atmosphere, and clouds over the lit image.
    var color = base * mix(1.0, shadow, 0.35) + transparency;
    if (comp_params.atmosphere_enabled != 0u) {
        color = mix(color, textureLoad(atmosphere_overlay, pixel, 0).rgb, textureLoad(atmosphere_overlay, pixel, 0).a);
    }
    if (comp_params.clouds_enabled != 0u) {
        let cloud = textureLoad(cloud_overlay, pixel, 0).rgb;
        let trans = textureLoad(cloud_transmittance, pixel, 0).r;
        color = color * trans + cloud;
    }
    textureStore(out_tex, pixel, vec4<f32>(color, raster_sample.a));
}
