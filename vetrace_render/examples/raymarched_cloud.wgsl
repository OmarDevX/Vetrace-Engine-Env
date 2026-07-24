// Object-bounded raymarched cloud for CustomShaderMaterial.
//
// This intentionally uses procedural noise so the example needs no textures.
// The current material ABI does not expose scene depth, so geometry intersecting
// the proxy volume cannot clip the raymarch. Objects fully behind it composite
// correctly through the cloud's transmittance.

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

struct Camera {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
};

struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> vetrace_custom: VetraceCustomParams;

@group(1) @binding(0)
var<uniform> camera: Camera;

const RAY_STEPS: u32 = 56u;
const BOX_MIN: vec3<f32> = vec3<f32>(-0.5);
const BOX_MAX: vec3<f32> = vec3<f32>(0.5);

fn hash31(p: vec3<f32>) -> f32 {
    var q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yzx + vec3<f32>(33.33));
    return fract((q.x + q.y) * q.z);
}

fn value_noise(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    var f = fract(p);
    f = f * f * (vec3<f32>(3.0) - 2.0 * f);

    let n000 = hash31(cell + vec3<f32>(0.0, 0.0, 0.0));
    let n100 = hash31(cell + vec3<f32>(1.0, 0.0, 0.0));
    let n010 = hash31(cell + vec3<f32>(0.0, 1.0, 0.0));
    let n110 = hash31(cell + vec3<f32>(1.0, 1.0, 0.0));
    let n001 = hash31(cell + vec3<f32>(0.0, 0.0, 1.0));
    let n101 = hash31(cell + vec3<f32>(1.0, 0.0, 1.0));
    let n011 = hash31(cell + vec3<f32>(0.0, 1.0, 1.0));
    let n111 = hash31(cell + vec3<f32>(1.0, 1.0, 1.0));

    let x00 = mix(n000, n100, f.x);
    let x10 = mix(n010, n110, f.x);
    let x01 = mix(n001, n101, f.x);
    let x11 = mix(n011, n111, f.x);
    return mix(mix(x00, x10, f.y), mix(x01, x11, f.y), f.z);
}

fn fbm(p_in: vec3<f32>) -> f32 {
    var p = p_in;
    var value = 0.0;
    var amplitude = 0.55;
    for (var octave = 0u; octave < 3u; octave += 1u) {
        value += value_noise(p) * amplitude;
        p = p * 2.03 + vec3<f32>(17.1, 11.7, 7.3);
        amplitude *= 0.5;
    }
    return value;
}

fn intersect_unit_box(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    let direction_sign = select(vec3<f32>(-1.0), vec3<f32>(1.0), direction >= vec3<f32>(0.0));
    let safe_direction = direction_sign * max(abs(direction), vec3<f32>(0.00001));
    let t0 = (BOX_MIN - origin) / safe_direction;
    let t1 = (BOX_MAX - origin) / safe_direction;
    let near3 = min(t0, t1);
    let far3 = max(t0, t1);
    let near_t = max(near3.x, max(near3.y, near3.z));
    let far_t = min(far3.x, min(far3.y, far3.z));
    return vec2<f32>(near_t, far_t);
}

fn sample_density(local_position: vec3<f32>, time: f32) -> f32 {
    let coverage = clamp(vetrace_custom.params[0].x, 0.05, 0.9);
    let noise_scale = max(vetrace_custom.params[0].z, 0.1);
    let wind_speed = vetrace_custom.params[0].w;
    let wind = vec3<f32>(time * wind_speed, time * wind_speed * 0.11, -time * wind_speed * 0.32);

    let radial = max(abs(local_position.x), abs(local_position.z));
    let side_fade = 1.0 - smoothstep(0.34, 0.5, radial);
    let bottom_fade = smoothstep(-0.5, -0.28, local_position.y);
    let top_fade = 1.0 - smoothstep(0.18, 0.5, local_position.y);
    let shape = side_fade * bottom_fade * top_fade;

    let low_frequency = fbm(local_position * noise_scale + wind);
    let detail = value_noise(local_position * noise_scale * 5.3 - wind * 1.7);
    let cloud = smoothstep(coverage, min(coverage + 0.28, 0.99), low_frequency - detail * 0.12);
    return cloud * shape;
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // normal_model is inverse-transpose(model), therefore its transpose is the
    // world-to-local matrix needed for ray-box intersection.
    let world_to_local = transpose(vetrace_custom.normal_model);
    let local_camera = (world_to_local * vec4<f32>(camera.camera_position.xyz, 1.0)).xyz;
    let world_ray = normalize(input.world_position - camera.camera_position.xyz);
    let local_ray = normalize((world_to_local * vec4<f32>(world_ray, 0.0)).xyz);
    let hit = intersect_unit_box(local_camera, local_ray);
    let start_t = max(hit.x, 0.0);

    if (hit.y <= start_t) {
        discard;
    }

    let step_size = (hit.y - start_t) / f32(RAY_STEPS);
    let jitter = hash31(input.world_position * 71.37);
    var distance = start_t + step_size * jitter;
    var transmittance = 1.0;
    var accumulated_color = vec3<f32>(0.0);

    let time = vetrace_custom.time_health.x;
    let extinction = max(vetrace_custom.params[0].y, 0.1);
    let world_light_direction = normalize(-vetrace_custom.light_direction_intensity.xyz);
    let light_direction = normalize((world_to_local * vec4<f32>(world_light_direction, 0.0)).xyz);
    let light_color = max(vetrace_custom.light_color_ambient.rgb, vec3<f32>(0.0));
    let light_intensity = max(vetrace_custom.light_direction_intensity.w, 0.0);
    let view_to_light = clamp(dot(-local_ray, light_direction), -1.0, 1.0);
    let forward_scatter = 0.55 + 0.45 * pow(max(view_to_light, 0.0), 5.0);

    for (var step = 0u; step < RAY_STEPS; step += 1u) {
        let p = local_camera + local_ray * distance;
        let density = sample_density(p, time);
        if (density > 0.002) {
            // One offset density lookup gives a cheap self-shadowing cue without
            // nesting another full light raymarch inside every view-ray step.
            let light_density = sample_density(p + light_direction * 0.12, time);
            let light_visibility = exp(-light_density * 1.8);
            let shadow_color = vetrace_custom.color_a.rgb;
            let sun_color = vetrace_custom.color_b.rgb * light_color;
            let lighting = mix(shadow_color, sun_color, light_visibility * forward_scatter);
            let step_alpha = 1.0 - exp(-density * extinction * step_size);
            accumulated_color += transmittance * step_alpha * lighting * (0.65 + light_intensity * 0.22);
            transmittance *= 1.0 - step_alpha;
            if (transmittance < 0.015) {
                break;
            }
        }
        distance += step_size;
    }

    let alpha = clamp(1.0 - transmittance, 0.0, 0.96);
    if (alpha < 0.002) {
        discard;
    }

    // The render pipeline uses straight-alpha blending. Convert the accumulated
    // premultiplied radiance back to straight color before returning it.
    let straight_color = accumulated_color / max(alpha, 0.0001);
    return vec4<f32>(clamp(straight_color, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}
