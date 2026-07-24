use super::*;

// Split-out implementation details for `wgpu_backend.rs`.

pub const OUTLINE_PASS_WGSL: &str = r#"
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    // Keep this naga/wgpu-0.20 friendly: do not dynamically index a local array
    // made in the shader.
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

struct OutlineParams {
    color: vec4<f32>,
    texel_size_thickness: vec4<f32>,
};

@group(0) @binding(0) var outline_mask: texture_2d<f32>;
@group(0) @binding(1) var outline_sampler: sampler;
@group(0) @binding(2) var<uniform> params: OutlineParams;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let texel = params.texel_size_thickness.xy;
    let radius = max(1.0, params.texel_size_thickness.z);
    let center = textureSample(outline_mask, outline_sampler, in.uv).a;
    var neighbor = 0.0;
    for (var y: i32 = -2; y <= 2; y = y + 1) {
        for (var x: i32 = -2; x <= 2; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel * radius;
            neighbor = max(neighbor, textureSample(outline_mask, outline_sampler, in.uv + offset).a);
        }
    }
    let edge = max(neighbor - center, 0.0);
    return vec4<f32>(params.color.rgb, params.color.a * edge);
}
"#;
