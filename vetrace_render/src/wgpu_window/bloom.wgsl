// Generic compact single-pass bloom. Bright neighborhoods are
// gathered in several rings and added back to the already tone-mapped scene.
struct CustomPostProcessUniform {
    p0: vec4<f32>, // threshold, intensity, radius pixels, enabled
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

@group(0) @binding(0) var scene_color: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@group(0) @binding(3) var<uniform> post: CustomPostProcessUniform;

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

fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

fn bright_sample(uv: vec2<f32>, threshold: f32) -> vec3<f32> {
    let color = textureSampleLevel(scene_color, scene_sampler, clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)), 0.0).rgb;
    let upper_threshold = max(threshold + 0.001, min(threshold + 0.28, 1.0));
    let amount = smoothstep(threshold, upper_threshold, luminance(color));
    return color * amount;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let center = textureSample(scene_color, scene_sampler, input.uv);
    if (post.p0.w < 0.5) {
        return center;
    }
    let threshold = clamp(post.p0.x, 0.0, 1.0);
    let intensity = max(post.p0.y, 0.0);
    let radius = max(post.p0.z, 0.5);
    let texel = 1.0 / max(vec2<f32>(textureDimensions(scene_color)), vec2<f32>(1.0));

    var glow = bright_sample(input.uv, threshold) * 0.16;
    var weight = 0.16;
    // Naga/WGPU 0.20 requires fixed-size local arrays to be indexed by a
    // compile-time constant. Keep the ring loop dynamic, but explicitly
    // unroll the eight sample directions so the shader validates on Vulkan.
    for (var ring: i32 = 1; ring <= 3; ring = ring + 1) {
        let ring_scale = radius * f32(ring) * 1.75;
        let ring_weight = 1.0 / (1.0 + f32(ring) * 1.25);
        let sample_offset = texel * ring_scale;

        glow += bright_sample(input.uv + vec2<f32>( 1.0,    0.0) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>(-1.0,    0.0) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>( 0.0,    1.0) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>( 0.0,   -1.0) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>( 0.707,  0.707) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>(-0.707,  0.707) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>( 0.707, -0.707) * sample_offset, threshold) * ring_weight;
        glow += bright_sample(input.uv + vec2<f32>(-0.707, -0.707) * sample_offset, threshold) * ring_weight;
        weight += ring_weight * 8.0;
    }
    glow /= max(weight, 1.0e-5);
    return vec4<f32>(center.rgb + glow * intensity, center.a);
}
