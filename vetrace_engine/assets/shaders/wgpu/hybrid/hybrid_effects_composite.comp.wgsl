// Production-active decomposed hybrid effects compositor.
// Binding 0 is the lit raster color_texture, not raw G-buffer albedo.
@group(0) @binding(0) var raster_direct_tex: texture_2d<f32>;
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
    _pad: u32,
};
@group(0) @binding(4) var<uniform> comp_params: CompositeParams;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(out_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    let raster_sample = textureLoad(raster_direct_tex, pixel, 0);
    let sky_t = clamp(f32(id.y) / max(f32(dims.y), 1.0), 0.0, 1.0);
    let sky = mix(vec3<f32>(0.52, 0.68, 0.92), vec3<f32>(0.02, 0.025, 0.04), sky_t);
    let base = select(sky, raster_sample.rgb, raster_sample.a > 0.0);
    let gi_uv = vec2<i32>(i32(floor(f32(id.x) * 0.5) * 2.0), i32(floor(f32(id.y) * 0.5) * 2.0));
    // GI resolve owns method selection and RTGI temporal/spatial filtering; composite only consumes gi_buffer.
    let resolved_gi = textureLoad(resolved_gi_buffer, gi_uv, 0).rgb;
    let ao = clamp(textureLoad(ambient_occlusion_tex, pixel, 0).r, 0.0, 1.0);
    let blended_gi = resolved_gi * ao;
    let shadow = select(1.0, textureLoad(rt_shadow_mask, pixel, 0).r, comp_params.rt_shadows_enabled != 0u);
    let mat = textureLoad(gbuf_material, pixel, 0);
    let roughness = clamp(f32(mat.g) / 255.0, 0.04, 1.0);
    let reflection_weight = (1.0 - roughness) * (1.0 - roughness);
    let ssr = textureLoad(ssr_reflection_radiance, pixel, 0);
    let rt = textureLoad(rt_reflection_radiance, pixel, 0);
    let ssr_conf = select(0.0, ssr.a, comp_params.ssr_enabled != 0u);
    let rt_conf = select(0.0, rt.a, comp_params.rt_reflections_enabled != 0u);
    let probe = base * 0.08 * (1.0 - roughness);
    let fallback = mix(probe, rt.rgb, rt_conf);
    let reflections = mix(fallback, ssr.rgb, ssr_conf) * reflection_weight * (1.0 - roughness * 0.35);
    let transparency = select(vec3<f32>(0.0), textureLoad(rt_transparency_radiance, pixel, 0).rgb, comp_params.rt_transparency_enabled != 0u);
    var color = base * shadow + blended_gi + reflections + transparency;
    if (comp_params.atmosphere_enabled != 0u) {
        color = mix(color, textureLoad(atmosphere_overlay, pixel, 0).rgb, textureLoad(atmosphere_overlay, pixel, 0).a);
    }
    if (comp_params.clouds_enabled != 0u) {
        let cloud = textureLoad(cloud_overlay, pixel, 0).rgb;
        let trans = textureLoad(cloud_transmittance, pixel, 0).r;
        color = color * trans + cloud;
    }
    textureStore(out_tex, pixel, vec4<f32>(color, 1.0));
}
