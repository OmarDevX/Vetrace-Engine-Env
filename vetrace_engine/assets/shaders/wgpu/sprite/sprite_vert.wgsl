struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) frag_uv: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> view_proj: mat4x4<f32>;

@vertex
fn vs_main(@location(0) in_pos: vec3<f32>,
           @location(1) in_uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    let clip = view_proj * vec4<f32>(in_pos, 1.0);
    out.clip_pos = clip;
    out.frag_uv = in_uv;
    return out;
}
