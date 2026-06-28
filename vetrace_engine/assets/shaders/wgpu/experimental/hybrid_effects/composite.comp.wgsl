// Production-active decomposed hybrid effects compositor.
@group(0) @binding(0) var raster_direct_tex: texture_2d<f32>;
@group(0) @binding(1) var out_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var baked_gi_buffer: texture_2d<f32>;
@group(0) @binding(3) var gi_history: texture_storage_2d<rgba16float, read_write>;
@group(0) @binding(5) var rt_shadow_mask: texture_2d<f32>;
@group(0) @binding(6) var rt_reflection_radiance: texture_2d<f32>;
@group(0) @binding(7) var rt_gi_radiance: texture_2d<f32>;
@group(0) @binding(8) var rt_transparency_radiance: texture_2d<f32>;
@group(0) @binding(9) var atmosphere_overlay: texture_2d<f32>;
@group(0) @binding(10) var cloud_overlay: texture_2d<f32>;
@group(0) @binding(11) var cloud_transmittance: texture_2d<f32>;
@group(0) @binding(12) var gbuf_material: texture_2d<u32>;

struct CompositeParams {
    temporal_blend: f32,
    rt_gi_enabled: u32,
    rt_reflections_enabled: u32,
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
    let base = textureLoad(raster_direct_tex, pixel, 0).rgb;
    let gi_uv = vec2<i32>(i32(floor(f32(id.x) * 0.5) * 2.0), i32(floor(f32(id.y) * 0.5) * 2.0));
    let baked_gi = textureLoad(baked_gi_buffer, gi_uv, 0).rgb;
    let rt_gi = select(vec3<f32>(0.0), textureLoad(rt_gi_radiance, pixel, 0).rgb, comp_params.rt_gi_enabled != 0u);
    let hist_gi = textureLoad(gi_history, gi_uv).rgb;
    let blended_gi = mix(baked_gi + rt_gi, hist_gi, comp_params.temporal_blend);
    let shadow = select(1.0, textureLoad(rt_shadow_mask, pixel, 0).r, comp_params.rt_shadows_enabled != 0u);
    let mat = textureLoad(gbuf_material, pixel, 0);
    let roughness = clamp(f32(mat.g) / 255.0, 0.04, 1.0);
    let reflection_weight = (1.0 - roughness) * (1.0 - roughness);
    let reflections = select(vec3<f32>(0.0), textureLoad(rt_reflection_radiance, pixel, 0).rgb * reflection_weight, comp_params.rt_reflections_enabled != 0u);
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
    textureStore(gi_history, gi_uv, vec4<f32>(blended_gi, 1.0));
}
