struct CustomPostProcessUniform {
    p0: vec4<f32>,
    p1: vec4<f32>,
    p2: vec4<f32>,
    p3: vec4<f32>,
    p4: vec4<f32>,
    p5: vec4<f32>,
    p6: vec4<f32>,
    p7: vec4<f32>,
    screen_time: vec4<f32>,
    info: vec4<f32>,
};

@group(0) @binding(0)
var scene_color: texture_2d<f32>;

@group(0) @binding(1)
var scene_sampler: sampler;

@group(0) @binding(2)
var scene_depth: texture_depth_2d;

@group(0) @binding(3)
var<uniform> post: CustomPostProcessUniform;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    var p = vec2<f32>(-1.0, -3.0);
    if (vertex_index == 1u) { p = vec2<f32>(3.0, 1.0); }
    if (vertex_index == 2u) { p = vec2<f32>(-1.0, 1.0); }
    return p;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let p = fullscreen_triangle_position(vertex_index);
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = p * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(scene_color, scene_sampler, input.uv);
    let strength = clamp(post.p0.x, 0.0, 1.0);
    let inner = clamp(post.p0.y, 0.0, 1.0);
    let outer = max(post.p0.z, inner + 0.001);
    let d = distance(input.uv, vec2<f32>(0.5, 0.5));
    let vignette = 1.0 - smoothstep(inner, outer, d) * strength;
    return vec4<f32>(color.rgb * vignette, color.a);
}
