use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) const SSAO_WGSL: &str = r#"
struct SsaoParams {
    params0: vec4<f32>, // width, height, radius pixels, intensity
    params1: vec4<f32>, // bias, sample count, near, far
    params2: vec4<f32>, // blur radius, reserved
};

@group(0) @binding(0)
var scene_depth: texture_depth_2d;

@group(0) @binding(1)
var<uniform> ssao: SsaoParams;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    // Avoid dynamic indexing into a local array here. Older naga/wgpu validation
    // can reject `pos[vertex_index]` for shader-created arrays.
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

fn depth_at_uv(uv: vec2<f32>) -> f32 {
    let dims_u = textureDimensions(scene_depth);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let safe_uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let coord = clamp(vec2<i32>(safe_uv * vec2<f32>(f32(dims.x), f32(dims.y))), vec2<i32>(0), dims - vec2<i32>(1));
    return textureLoad(scene_depth, coord, 0);
}

fn tap_offset(index: i32) -> vec2<f32> {
    var offset = vec2<f32>(-0.326212, -0.405810);
    if (index == 1) { offset = vec2<f32>(-0.840144, -0.073580); }
    if (index == 2) { offset = vec2<f32>(-0.695914,  0.457137); }
    if (index == 3) { offset = vec2<f32>(-0.203345,  0.620716); }
    if (index == 4) { offset = vec2<f32>( 0.962340, -0.194983); }
    if (index == 5) { offset = vec2<f32>( 0.473434, -0.480026); }
    if (index == 6) { offset = vec2<f32>( 0.519456,  0.767022); }
    if (index == 7) { offset = vec2<f32>( 0.185461, -0.893124); }
    if (index == 8) { offset = vec2<f32>( 0.507431,  0.064425); }
    if (index == 9) { offset = vec2<f32>( 0.896420,  0.412458); }
    if (index == 10) { offset = vec2<f32>(-0.321940, -0.932615); }
    if (index == 11) { offset = vec2<f32>(-0.791559, -0.597710); }
    return offset;
}

fn interleaved_gradient_noise(pixel: vec2<f32>) -> f32 {
    return fract(52.9829189 * fract(dot(pixel, vec2<f32>(0.06711056, 0.00583715))));
}

fn rotate_offset(offset: vec2<f32>, angle: f32) -> vec2<f32> {
    let s = sin(angle);
    let c = cos(angle);
    return vec2<f32>(offset.x * c - offset.y * s, offset.x * s + offset.y * c);
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let center = depth_at_uv(input.uv);
    if (center >= 0.99999) {
        return vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }

    let resolution = max(ssao.params0.xy, vec2<f32>(1.0));
    let radius_pixels = clamp(ssao.params0.z, 1.0, 64.0);
    let intensity = max(ssao.params0.w, 0.0);
    let bias = max(ssao.params1.x, 0.00001);
    let tap_count = i32(clamp(ssao.params1.y, 4.0, 12.0));
    let pixel = input.uv * resolution;
    let angle = interleaved_gradient_noise(pixel) * 6.28318530718;

    var occlusion = 0.0;
    var weight_sum = 0.0;
    for (var i: i32 = 0; i < 12; i = i + 1) {
        if (i < tap_count) {
            let dir = rotate_offset(tap_offset(i), angle);
            let sample_uv = input.uv + dir * radius_pixels / resolution;
            let sample_depth = depth_at_uv(sample_uv);
            // WGPU depth is smaller when the sampled point is closer to the camera.
            // A closer neighbor near this pixel means this point is tucked behind nearby geometry.
            let dz = center - sample_depth;
            let close_enough = 1.0 - smoothstep(0.0, 0.08, abs(dz));
            let contributes = select(0.0, 1.0, dz > bias && sample_depth < 0.99999);
            let w = close_enough * contributes;
            occlusion = occlusion + w;
            weight_sum = weight_sum + 1.0;
        }
    }

    let normalized = occlusion / max(weight_sum, 1.0);
    let ao = clamp(1.0 - normalized * intensity, 0.0, 1.0);
    return vec4<f32>(ao, ao, ao, 1.0);
}
"#;

pub(super) const SSAO_BLUR_WGSL: &str = r#"
struct SsaoParams {
    params0: vec4<f32>,
    params1: vec4<f32>,
    params2: vec4<f32>,
};

@group(0) @binding(0)
var ao_texture: texture_2d<f32>;

@group(0) @binding(1)
var scene_depth: texture_depth_2d;

@group(0) @binding(2)
var screen_sampler: sampler;

@group(0) @binding(3)
var<uniform> ssao: SsaoParams;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    // Avoid dynamic indexing into a local array here. Older naga/wgpu validation
    // can reject `pos[vertex_index]` for shader-created arrays.
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

fn depth_at_uv(uv: vec2<f32>) -> f32 {
    let dims_u = textureDimensions(scene_depth);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let safe_uv = clamp(uv, vec2<f32>(0.0), vec2<f32>(0.999999));
    let coord = clamp(vec2<i32>(safe_uv * vec2<f32>(f32(dims.x), f32(dims.y))), vec2<i32>(0), dims - vec2<i32>(1));
    return textureLoad(scene_depth, coord, 0);
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let resolution = max(ssao.params0.xy, vec2<f32>(1.0));
    let texel = 1.0 / resolution;
    let radius = clamp(ssao.params2.x, 0.0, 4.0);
    if (radius < 0.25) {
        let ao = textureSample(ao_texture, screen_sampler, input.uv).r;
        return vec4<f32>(ao, ao, ao, 1.0);
    }

    let center_depth = depth_at_uv(input.uv);
    var sum = 0.0;
    var weight_sum = 0.0;
    for (var y: i32 = -2; y <= 2; y = y + 1) {
        for (var x: i32 = -2; x <= 2; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y));
            let dist = length(offset);
            if (dist <= radius + 0.001) {
                let uv = input.uv + offset * texel;
                let sample_depth = depth_at_uv(uv);
                let depth_weight = 1.0 - smoothstep(0.0, 0.03, abs(sample_depth - center_depth));
                let spatial_weight = 1.0 / (1.0 + dist);
                let weight = max(depth_weight * spatial_weight, 0.0001);
                let ao = textureSample(ao_texture, screen_sampler, uv).r;
                sum = sum + ao * weight;
                weight_sum = weight_sum + weight;
            }
        }
    }
    let blurred = clamp(sum / max(weight_sum, 0.0001), 0.0, 1.0);
    return vec4<f32>(blurred, blurred, blurred, 1.0);
}
"#;

pub(super) const SSAO_COMPOSITE_WGSL: &str = r#"
struct SsaoParams {
    params0: vec4<f32>,
    params1: vec4<f32>,
    params2: vec4<f32>,
};

@group(0) @binding(0)
var scene_color: texture_2d<f32>;

@group(0) @binding(1)
var ao_texture: texture_2d<f32>;

@group(0) @binding(2)
var screen_sampler: sampler;

@group(0) @binding(3)
var<uniform> ssao: SsaoParams;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    // Avoid dynamic indexing into a local array here. Older naga/wgpu validation
    // can reject `pos[vertex_index]` for shader-created arrays.
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
    let color = textureSample(scene_color, screen_sampler, input.uv);
    let ao = textureSample(ao_texture, screen_sampler, input.uv).r;
    // Post-composite AO is intentionally conservative: do not crush the image
    // fully to black, and leave alpha untouched for future compositor paths.
    let factor = mix(0.65, 1.0, clamp(ao, 0.0, 1.0));
    return vec4<f32>(color.rgb * factor, color.a);
}
"#;
