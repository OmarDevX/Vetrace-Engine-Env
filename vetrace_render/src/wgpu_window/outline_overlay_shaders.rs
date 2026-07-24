use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) const OUTLINE_MASK_FRAGMENT_WGSL: &str = r#"
struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@fragment
fn fs_main(_input: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
"#;

pub(super) const OUTLINE_FRAGMENT_WGSL: &str = r#"
struct VetraceCustomParams {
    params: array<vec4<f32>, 4>,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    time_health: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> vetrace_custom: VetraceCustomParams;

struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@fragment
fn fs_main(_input: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(vetrace_custom.color_a.rgb, 1.0);
}
"#;

pub(super) const OVERLAY_WGSL: &str = r#"
struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    return input.color;
}
"#;
