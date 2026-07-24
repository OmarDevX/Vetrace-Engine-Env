// Simple Shooter GPU custom material.
// vetrace_render's WGPU custom-shader cache binds this uniform at group(0)/binding(0).

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
    fog_color_density: vec4<f32>,
    fog_params: vec4<f32>,
    baked_lightmap_transform: vec4<f32>,
    baked_gi_params: vec4<f32>,
    baked_gi_extra: vec4<f32>,
    baked_probe_sh0: vec4<f32>,
    baked_probe_sh1: vec4<f32>,
    baked_probe_sh2: vec4<f32>,
    baked_probe_sh3: vec4<f32>,
    baked_probe_sh4: vec4<f32>,
    baked_probe_sh5: vec4<f32>,
    baked_probe_sh6: vec4<f32>,
    baked_probe_sh7: vec4<f32>,
    baked_probe_sh8: vec4<f32>,
    post_process_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> vetrace_custom: VetraceCustomParams;

const PI: f32 = 3.141592653589793;

struct MaterialFragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

fn baked_probe_irradiance(normal: vec3<f32>) -> vec3<f32> {
    if (vetrace_custom.baked_gi_params.y < 0.5) {
        return vec3<f32>(0.0);
    }
    let n = normalize(normal);
    let x = n.x;
    let y = n.y;
    let z = n.z;
    let irradiance =
        vetrace_custom.baked_probe_sh0.rgb * 0.282095
        + vetrace_custom.baked_probe_sh1.rgb * (0.488603 * y)
        + vetrace_custom.baked_probe_sh2.rgb * (0.488603 * z)
        + vetrace_custom.baked_probe_sh3.rgb * (0.488603 * x)
        + vetrace_custom.baked_probe_sh4.rgb * (1.092548 * x * y)
        + vetrace_custom.baked_probe_sh5.rgb * (1.092548 * y * z)
        + vetrace_custom.baked_probe_sh6.rgb * (0.315392 * (3.0 * z * z - 1.0))
        + vetrace_custom.baked_probe_sh7.rgb * (1.092548 * x * z)
        + vetrace_custom.baked_probe_sh8.rgb * (0.546274 * (x * x - y * y));
    return max(irradiance, vec3<f32>(0.0));
}

fn lambert_light(normal: vec3<f32>) -> vec3<f32> {
    let n = normalize(normal);
    let to_light = normalize(-vetrace_custom.light_direction_intensity.xyz);
    let ndotl = max(dot(n, to_light), 0.0);
    let ambient = clamp(vetrace_custom.light_color_ambient.a, 0.0, 1.0);
    let intensity = max(vetrace_custom.light_direction_intensity.w, 0.0);
    return vec3<f32>(ambient) + baked_probe_irradiance(n) / PI + vetrace_custom.light_color_ambient.rgb * ndotl * intensity;
}

@fragment
fn fs_main(input: MaterialFragmentInput) -> @location(0) vec4<f32> {
    let seed = vetrace_custom.params[0].x;
    let time = vetrace_custom.time_health.x;
    let health01 = clamp(vetrace_custom.time_health.y, 0.0, 1.0);
    let t = 0.5 + 0.5 * sin(input.world_position.y * 2.5 + time * 3.0 + seed);
    let gradient = mix(vetrace_custom.color_a.rgb, vetrace_custom.color_b.rgb, t) * (0.35 + health01 * 0.65);
    if (vetrace_custom.baked_gi_extra.x >= 2.5) {
        let probe = baked_probe_irradiance(input.normal);
        return vec4(probe / (vec3<f32>(1.0) + probe), 1.0);
    }
    let lit = gradient * lambert_light(input.normal);
    let rim = pow(1.0 - abs(dot(normalize(input.normal), vec3<f32>(0.0, 0.0, 1.0))), 2.0) * 0.10;
    return vec4<f32>(clamp(lit + gradient * rim, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
