use super::*;

pub(super) const SKY_WGSL: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
    inverse_view_proj: mat4x4<f32>,
};

struct EnvironmentUniform {
    slots_counts: vec4<u32>,
    params0: vec4<f32>,
    params1: vec4<f32>,
    post_process: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var environment_cubemaps: texture_cube_array<f32>;

@group(1) @binding(1)
var environment_sampler: sampler;

@group(1) @binding(3)
var<uniform> environment: EnvironmentUniform;

struct SkyVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) clip_xy: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> SkyVertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var output: SkyVertexOutput;
    output.clip_xy = positions[vertex_index];
    output.position = vec4<f32>(output.clip_xy, 1.0, 1.0);
    return output;
}

fn rotate_direction(direction: vec3<f32>, radians: f32) -> vec3<f32> {
    let sine = sin(radians);
    let cosine = cos(radians);
    return vec3<f32>(
        direction.x * cosine - direction.z * sine,
        direction.y,
        direction.x * sine + direction.z * cosine,
    );
}

fn smooth_transition(value: f32) -> f32 {
    let t = clamp(value, 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn sample_slot(slot: u32, direction: vec3<f32>) -> vec3<f32> {
    if (slot == 0u) {
        return vec3<f32>(0.0);
    }
    return textureSampleLevel(environment_cubemaps, environment_sampler, direction, i32(slot), 0.0).rgb;
}

fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + vec3<f32>(b))) / (color * (c * color + vec3<f32>(d)) + vec3<f32>(e)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn neutral_filmic(color: vec3<f32>) -> vec3<f32> {
    let x = max(color - vec3<f32>(0.004), vec3<f32>(0.0));
    return clamp((x * (6.2 * x + vec3<f32>(0.5))) / (x * (6.2 * x + vec3<f32>(1.7)) + vec3<f32>(0.06)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_tonemap(color: vec3<f32>) -> vec3<f32> {
    if (environment.params1.w >= 0.5) {
        return max(color, vec3<f32>(0.0));
    }
    let exposed = max(color * max(environment.post_process.x, 0.0001), vec3<f32>(0.0));
    let mode = i32(clamp(environment.post_process.z, 0.0, 3.0));
    var mapped = exposed;
    if (mode == 1) {
        mapped = aces_filmic(exposed);
    } else if (mode == 2) {
        mapped = neutral_filmic(exposed);
    } else if (mode == 3) {
        mapped = exposed / (vec3<f32>(1.0) + exposed);
    }
    let gamma_adjust = 2.2 / clamp(environment.post_process.y, 1.0, 3.0);
    return pow(clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(gamma_adjust));
}

@fragment
fn fs_main(input: SkyVertexOutput) -> @location(0) vec4<f32> {
    let far_world = camera.inverse_view_proj * vec4<f32>(input.clip_xy, 1.0, 1.0);
    let safe_w = select(0.00001, far_world.w, abs(far_world.w) > 0.00001);
    let world_position = far_world.xyz / safe_w;
    let direction = rotate_direction(normalize(world_position - camera.camera_position.xyz), environment.params0.z);
    let primary = sample_slot(environment.slots_counts.x, direction);
    let secondary = sample_slot(environment.slots_counts.y, direction);
    let radiance = mix(primary, secondary, smooth_transition(environment.params0.x)) * max(environment.params0.y, 0.0);
    return vec4<f32>(apply_tonemap(radiance), 1.0);
}
"#;

pub(super) const REFLECTION_PREFILTER_WGSL: &str = include_str!("../environment_prefilter.wgsl");
