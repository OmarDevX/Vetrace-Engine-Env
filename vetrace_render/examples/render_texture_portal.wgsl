// Minimal game-side material that displays RenderTextureCamera slot 0.
// The renderer rasterizes the scene into binding 11; this shader only decides
// how that image is mapped and styled on the receiving object.

struct VetraceCustomParams {
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

struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> vetrace_custom: VetraceCustomParams;

@group(0) @binding(2)
var render_view_sampler: sampler;

@group(0) @binding(11)
var render_view_0: texture_2d<f32>;

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(input.uv.x, 1.0 - input.uv.y);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        discard;
    }

    let view_color = textureSample(render_view_0, render_view_sampler, uv).rgb;
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let border = 1.0 - smoothstep(0.0, 0.025, edge);
    let border_color = vec3<f32>(0.08, 0.72, 1.0) * (1.5 + 0.25 * sin(vetrace_custom.time_health.x * 3.0));
    return vec4<f32>(mix(view_color, border_color, border), 1.0);
}
