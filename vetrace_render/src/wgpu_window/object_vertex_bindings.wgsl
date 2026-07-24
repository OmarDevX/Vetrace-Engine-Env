struct Camera {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
};

struct VetraceObjectParams {
    params: array<vec4<f32>, 4>,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    time_health: vec4<f32>,
    light_direction_intensity: vec4<f32>,
    light_color_ambient: vec4<f32>,
    pbr_params: vec4<f32>,
    pbr_extra: vec4<f32>,
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

@group(0) @binding(0)
var<uniform> object: VetraceObjectParams;

@group(1) @binding(0)
var<uniform> camera: Camera;
