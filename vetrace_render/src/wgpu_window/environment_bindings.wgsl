struct EnvironmentUniform {
    slots_counts: vec4<u32>,
    params0: vec4<f32>,
    params1: vec4<f32>,
    post_process: vec4<f32>,
};

struct ReflectionProbeGpu {
    world_to_probe: mat4x4<f32>,
    half_extents_blend: vec4<f32>,
    capture_intensity: vec4<f32>,
    slots_modes: vec4<u32>, // xy slots, z parallax mode, w priority bits
    transition_params: vec4<f32>,
    layer_masks: vec4<u32>,
};

struct ReflectionProbeBuffer {
    probes: array<ReflectionProbeGpu>,
};

@group(2) @binding(0)
var environment_cubemaps: texture_cube_array<f32>;

@group(2) @binding(1)
var environment_sampler: sampler;

@group(2) @binding(2)
var<storage, read> reflection_probe_buffer: ReflectionProbeBuffer;

@group(2) @binding(3)
var<uniform> environment: EnvironmentUniform;

@group(2) @binding(4)
var environment_brdf_lut: texture_2d<f32>;

@group(2) @binding(5)
var environment_brdf_sampler: sampler;
