use super::*;

// Split-out implementation details for `wgpu_window.rs`.

pub(super) const SHADOW_WGSL: &str = r#"
struct ShadowMaterial {
    params: array<vec4<f32>, 4>,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    time_health: vec4<f32>,
    light_direction_intensity: vec4<f32>,
    light_color_ambient: vec4<f32>,
    pbr_params: vec4<f32>,       // z = material alpha
    pbr_extra: vec4<f32>,        // z = alpha cutoff, w = alpha mode
    light_counts: vec4<f32>,
    directional_lights: array<vec4<f32>, 4>,
    directional_colors: array<vec4<f32>, 4>,
    point_lights: array<vec4<f32>, 8>,
    point_colors_ranges: array<vec4<f32>, 8>,
    spot_lights: array<vec4<f32>, 4>,
    spot_dirs_ranges: array<vec4<f32>, 4>,
    spot_colors_inner: array<vec4<f32>, 4>,
    spot_params: array<vec4<f32>, 4>,
    shadow_view_proj: mat4x4<f32>,
    shadow_params: vec4<f32>,
    shadow_cascade_view_proj: array<mat4x4<f32>, 4>,
    shadow_cascade_splits: vec4<f32>,
    shadow_extra: vec4<f32>,
    shadow_bias_extra: vec4<f32>,
    model: mat4x4<f32>,
    normal_model: mat4x4<f32>,
};

struct Camera {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> material: ShadowMaterial;

@group(0) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(0) @binding(2)
var material_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera: Camera;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let world_position = material.model * vec4<f32>(input.position, 1.0);
    out.position = camera.view_proj * world_position;
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) {
    let material_uv = input.uv * max(material.params[0].xy, vec2<f32>(0.0001));
    let base_tex = textureSample(base_color_texture, material_sampler, material_uv);
    let alpha = clamp(material.pbr_params.z * material.color_a.a * input.color.a * base_tex.a, 0.0, 1.0);
    let alpha_mode = material.pbr_extra.w;
    let alpha_cutoff = clamp(material.pbr_extra.z, 0.0, 1.0);
    if (alpha_mode > 0.5 && alpha_mode < 1.5 && alpha < alpha_cutoff) {
        discard;
    }
    if (alpha <= 0.001) {
        discard;
    }
}
"#;
