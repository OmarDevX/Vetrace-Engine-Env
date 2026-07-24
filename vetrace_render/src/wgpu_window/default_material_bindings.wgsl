
struct VetraceCustomParams {
    params: array<vec4<f32>, 4>,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    time_health: vec4<f32>,
    light_direction_intensity: vec4<f32>, // xyz = light travel direction, w = intensity
    light_color_ambient: vec4<f32>,       // rgb = light color, a = ambient floor
    pbr_params: vec4<f32>,                // x = roughness factor, y = metallic factor, z = alpha, w = flags
    pbr_extra: vec4<f32>,                 // x = normal scale, y = occlusion strength, z = alpha cutoff, w = alpha mode
    light_counts: vec4<f32>,              // x = directional count, y = point count, z = spot count, w = ambient floor
    directional_lights: array<vec4<f32>, 4>,       // xyz = travel direction, w = intensity
    directional_colors: array<vec4<f32>, 4>,       // rgb = color
    point_lights: array<vec4<f32>, 8>,             // xyz = position, w = intensity
    point_colors_ranges: array<vec4<f32>, 8>,      // rgb = color, w = range <= 0 unlimited
    spot_lights: array<vec4<f32>, 4>,              // xyz = position, w = intensity
    spot_dirs_ranges: array<vec4<f32>, 4>,         // xyz = emission direction, w = range <= 0 unlimited
    spot_colors_inner: array<vec4<f32>, 4>,        // rgb = color, w = cos(inner cone)
    spot_params: array<vec4<f32>, 4>,              // x = cos(outer cone)
    shadow_view_proj: mat4x4<f32>,                 // first cascade / legacy directional shadow projection
    shadow_params: vec4<f32>,                      // x enabled, y map size, z bias, w soft PCF radius
    shadow_cascade_view_proj: array<mat4x4<f32>, 4>,
    shadow_cascade_splits: vec4<f32>,              // camera-distance split ends
    shadow_extra: vec4<f32>,                       // x cascade count, y PCF quality, z filter mode, w PCSS light radius
    shadow_bias_extra: vec4<f32>,                  // x slope-scale bias, y normal-offset bias, z EVSM blur radius, w EVSM exponent
    model: mat4x4<f32>,
    normal_model: mat4x4<f32>,
    fog_color_density: vec4<f32>,          // rgb = fog tint/albedo, a = density
    fog_params: vec4<f32>,                 // x = enabled, y = anisotropy, z = sky distance, w = reserved
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
    reflection_probe_indices: vec4<u32>,
    reflection_probe_params: vec4<f32>,
};

struct Camera {
    view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> vetrace_custom: VetraceCustomParams;

@group(0) @binding(1)
var base_color_texture: texture_2d<f32>;

@group(0) @binding(2)
var material_sampler: sampler;

@group(0) @binding(3)
var normal_texture: texture_2d<f32>;

@group(0) @binding(4)
var metallic_roughness_texture: texture_2d<f32>;

@group(0) @binding(5)
var occlusion_texture: texture_2d<f32>;

@group(0) @binding(6)
var emissive_texture: texture_2d<f32>;

@group(0) @binding(7)
var directional_shadow_map: texture_depth_2d_array;

@group(0) @binding(8)
var directional_shadow_sampler: sampler_comparison;

@group(0) @binding(9)
var directional_evsm_moments: texture_2d_array<f32>;

@group(0) @binding(10)
var baked_lightmap_texture: texture_2d<f32>;

// Generic game-side render-to-texture camera outputs. Custom materials may
// sample these with material_sampler. Unused/missing slots are bound to black.
@group(0) @binding(11)
var custom_render_texture_0: texture_2d<f32>;

@group(0) @binding(12)
var custom_render_texture_1: texture_2d<f32>;

@group(0) @binding(13)
var custom_render_texture_2: texture_2d<f32>;

@group(0) @binding(14)
var custom_render_texture_3: texture_2d<f32>;

@group(0) @binding(15)
var custom_render_texture_sampler: sampler;

@group(1) @binding(0)
var<uniform> camera: Camera;

struct FragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @builtin(front_facing) front_facing: bool,
};

const PI: f32 = 3.141592653589793;

fn distribution_ggx(n: vec3<f32>, h: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let ndoth = max(dot(n, h), 0.0);
    let denom = (ndoth * ndoth) * (a2 - 1.0) + 1.0;
    return a2 / max(PI * denom * denom, 0.0001);
}

fn geometry_schlick_ggx(ndotv: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return ndotv / max(ndotv * (1.0 - k) + k, 0.0001);
}

fn geometry_smith(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, roughness: f32) -> f32 {
    let ndotv = max(dot(n, v), 0.0);
    let ndotl = max(dot(n, l), 0.0);
    return geometry_schlick_ggx(ndotv, roughness) * geometry_schlick_ggx(ndotl, roughness);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn fresnel_schlick_roughness(cos_theta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn normal_from_map(input: FragmentInput) -> vec3<f32> {
    let geometric_n = normalize(input.normal);
    let tangent = normalize(input.tangent.xyz);
    var bitangent = normalize(cross(geometric_n, tangent) * input.tangent.w);
    if (dot(bitangent, bitangent) < 0.0001) {
        bitangent = normalize(cross(geometric_n, vec3<f32>(0.0, 1.0, 0.0)));
    }
    let sampled = textureSample(normal_texture, material_sampler, input.uv * max(vetrace_custom.params[0].xy, vec2<f32>(0.0001))).xyz * 2.0 - vec3<f32>(1.0);
    let normal_scale = max(vetrace_custom.pbr_extra.x, 0.0);
    let tangent_space = normalize(vec3<f32>(sampled.xy * normal_scale, sampled.z));
    let tbn = mat3x3<f32>(tangent, bitangent, geometric_n);
    let mapped = normalize(tbn * tangent_space);
    if (dot(mapped, mapped) < 0.0001) {
        return geometric_n;
    }
    return mapped;
}

fn aces_filmic(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + vec3<f32>(b))) / (color * (c * color + vec3<f32>(d)) + vec3<f32>(e)), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn range_attenuation(distance: f32, range: f32) -> f32 {
    if (range <= 0.0) {
        return 1.0;
    }
    let x = clamp(1.0 - distance / max(range, 0.0001), 0.0, 1.0);
    return x * x;
}

fn choose_shadow_cascade(world_position: vec3<f32>) -> i32 {
    let cascade_count = i32(clamp(vetrace_custom.shadow_extra.x, 0.0, 4.0));
    if (cascade_count <= 0) {
        return -1;
    }
    let view_distance = dot(world_position - camera.camera_position.xyz, normalize(camera.camera_forward.xyz));
    if (view_distance < 0.0) {
        return -1;
    }
    if (cascade_count >= 1 && view_distance <= vetrace_custom.shadow_cascade_splits.x) {
        return 0;
    }
    if (cascade_count >= 2 && view_distance <= vetrace_custom.shadow_cascade_splits.y) {
        return 1;
    }
    if (cascade_count >= 3 && view_distance <= vetrace_custom.shadow_cascade_splits.z) {
        return 2;
    }
    if (cascade_count >= 4 && view_distance <= vetrace_custom.shadow_cascade_splits.w) {
        return 3;
    }
    return -1;
}

fn shadow_filter_mode() -> i32 {
    // 0 = hard, 1 = fixed PCF, 2 = PCSS/contact-hardening, 3 = full blurred EVSM moments.
    return i32(clamp(vetrace_custom.shadow_extra.z, 0.0, 3.0));
}

fn shadow_sample_count() -> i32 {
    let quality = i32(clamp(vetrace_custom.shadow_extra.y, 1.0, 3.0));
    return select(4, select(8, 12, quality >= 3), quality >= 2);
}

fn shadow_poisson_offset(index: i32) -> vec2<f32> {
    var offset = vec2<f32>(-0.326212, -0.40581);
