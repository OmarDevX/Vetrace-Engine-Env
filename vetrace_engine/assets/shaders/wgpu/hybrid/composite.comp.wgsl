@group(0) @binding(0) var color_tex: texture_storage_2d<rgba16float, read>;
@group(0) @binding(1) var out_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var gi_buffer: texture_2d<f32>;
@group(0) @binding(3) var gi_history: texture_storage_2d<rgba16float, read_write>;

struct CompositeParams { temporal_blend: f32, _pad: vec3<f32> };
@group(0) @binding(4) var<uniform> comp_params: CompositeParams;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(out_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let col = textureLoad(color_tex, vec2<i32>(id.xy));
    let gi_uv = vec2<i32>(i32(floor(f32(id.x) * 0.5) * 2.0), i32(floor(f32(id.y) * 0.5) * 2.0));
    let cur_gi = textureLoad(gi_buffer, gi_uv, 0).rgb;
    let hist_gi = textureLoad(gi_history, gi_uv).rgb;
    let blended = mix(cur_gi, hist_gi, comp_params.temporal_blend);
    textureStore(out_tex, vec2<i32>(id.xy), vec4<f32>(col.rgb + blended, 1.0));
    textureStore(gi_history, gi_uv, vec4<f32>(blended, 1.0));
}