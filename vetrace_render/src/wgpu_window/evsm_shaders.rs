use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) const EVSM_MOMENT_WGSL: &str = r#"
struct EvsmPassUniform {
    direction_radius_layer: vec4<f32>,
    exponent_size: vec4<f32>,
};

@group(0) @binding(0)
var source_depth: texture_depth_2d_array;

@group(0) @binding(1)
var<uniform> evsm_pass: EvsmPassUniform;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn evsm_warp_depth(depth: f32, exponent: f32) -> vec4<f32> {
    let d = clamp(depth, 0.0, 1.0);
    let safe_exponent = clamp(exponent, 1.0, 5.5);
    let positive = exp(clamp(safe_exponent * d, -5.5, 5.5));
    let negative = -exp(clamp(-safe_exponent * d, -5.5, 5.5));
    return vec4<f32>(positive, positive * positive, negative, negative * negative);
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let dims_u = textureDimensions(source_depth);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let coord = clamp(vec2<i32>(input.uv * vec2<f32>(f32(dims.x), f32(dims.y))), vec2<i32>(0), dims - vec2<i32>(1));
    let layer = i32(clamp(evsm_pass.direction_radius_layer.w, 0.0, 3.0));
    let depth = textureLoad(source_depth, coord, layer, 0);
    return evsm_warp_depth(depth, evsm_pass.exponent_size.x);
}
"#;

pub(super) const EVSM_BLUR_WGSL: &str = r#"
struct EvsmPassUniform {
    direction_radius_layer: vec4<f32>,
    exponent_size: vec4<f32>,
};

@group(0) @binding(0)
var source_moments: texture_2d_array<f32>;

@group(0) @binding(1)
var<uniform> evsm_pass: EvsmPassUniform;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    out.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    out.uv = uv;
    return out;
}

fn gaussian_weight(index: i32) -> f32 {
    var w = 0.2270270270;
    if (abs(index) == 1) { w = 0.1945945946; }
    if (abs(index) == 2) { w = 0.1216216216; }
    if (abs(index) == 3) { w = 0.0540540541; }
    if (abs(index) == 4) { w = 0.0162162162; }
    return w;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let dims_u = textureDimensions(source_moments);
    let dims = vec2<i32>(i32(dims_u.x), i32(dims_u.y));
    let base = clamp(vec2<i32>(input.uv * vec2<f32>(f32(dims.x), f32(dims.y))), vec2<i32>(0), dims - vec2<i32>(1));
    let layer = i32(clamp(evsm_pass.direction_radius_layer.w, 0.0, 3.0));
    let direction = evsm_pass.direction_radius_layer.xy;
    let radius = clamp(evsm_pass.direction_radius_layer.z, 0.0, 8.0);
    let step_scale = radius / 4.0;

    var sum = vec4<f32>(0.0);
    var weight_sum = 0.0;
    for (var i: i32 = -4; i <= 4; i = i + 1) {
        let w = gaussian_weight(i);
        let offset = vec2<i32>(round(direction * f32(i) * step_scale));
        let coord = clamp(base + offset, vec2<i32>(0), dims - vec2<i32>(1));
        sum = sum + textureLoad(source_moments, coord, layer, 0) * w;
        weight_sum = weight_sum + w;
    }
    return sum / max(weight_sum, 0.0001);
}
"#;
