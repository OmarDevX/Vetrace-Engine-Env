// Production decomposed hybrid ray-traced ambient occlusion pass.
const T_EPS: f32 = 0.001;
const INF_T: f32 = 1.0e20;
struct Params {
    camera_pos: vec4<f32>, camera_front: vec4<f32>, camera_up: vec4<f32>, camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>, fov: f32, num_objects: i32, is_fisheye: i32, _pad0: i32,
    skycolor: vec4<f32>, taa_jitter: vec2<f32>, current_time: f32, frame_number: i32,
    selected_index: i32, max_bounces: i32, light_samples: i32, dir_shadow_samples: i32,
    shadow_mode: u32, raytraced_shadows_enabled: u32, shadow_quality: u32, max_shadow_rays: u32,
    emissive_shadow_samples: u32, directional_shadow_samples: u32, cloud_object_shadows_enabled: u32,
    max_rt_shadow_distance: f32, rt_shadow_ray_t_max: f32, min_soft_shadow_radius: f32,
    raytraced_reflections_enabled: u32, _pad_reflections: u32,
    inv_view_proj: mat4x4<f32>, prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>, dir_light_color: vec4<f32>, sky_occlusion: f32,
    total_triangles: u32, total_bvh_nodes: u32, total_tri_bvh_nodes: u32,
    dof_aperture: f32, dof_focus_dist: f32, dof_enable: u32, _pad_dof: u32,
    atmosphere: u32, atmo_count: u32, cloud_count: u32, atmosphere_mode: u32,
    atmosphere_sun_controls: vec4<f32>,
    cloud_history_weight: f32, cloud_sample_count: u32, cloud_temporal_quality: u32, cloud_shadow_mode: u32,
    renderer_mode: u32, rt_debug_view: u32, rt_debug_counters: u32, max_traversal_steps: u32,
    max_transparent_surfaces: u32, shadow_max_distance: f32, reflection_max_distance: f32, gi_max_distance: f32,
    min_ray_offset: f32,
};

struct RtEffectParams {
    inv_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    enabled: u32,
    mode: u32,
    gi_mode: u32,
    rtao_sample_count: u32,
    rtao_radius_bits: u32,
    _pad_rt: u32,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<u32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var ao_out: texture_storage_2d<r16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;
// Shared BVH declarations/traversal are concatenated by Rust from hybrid/bvh_traversal.wgsl.
fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var world = rt_params.inv_view_proj * vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    return (world / max(world.w, 1.0e-6)).xyz;
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn hash11(n: u32) -> f32 {
    var x = n;
    x = (x ^ 61u) ^ (x >> 16u);
    x = x * 9u;
    x = x ^ (x >> 4u);
    x = x * 0x27d4eb2du;
    x = x ^ (x >> 15u);
    return f32(x & 0x00ffffffu) / 16777215.0;
}

fn tangent_basis(n: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.95);
    let t = normalize(cross(up, n));
    let b = cross(n, t);
    return mat3x3<f32>(t, b, n);
}

fn cosine_hemisphere(u1: f32, u2: f32, n: vec3<f32>) -> vec3<f32> {
    let r = sqrt(u1);
    let phi = 6.28318530718 * u2;
    let local = vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - u1)));
    return normalize(tangent_basis(n) * local);
}

fn ao_estimate(px: vec2<i32>, dims: vec2<u32>, sample_count: u32, radius: f32, seed_base: u32) -> f32 {
    let depth = textureLoad(depth_tex, px, 0).x;
    if (depth >= 0.9999) { return 1.0; }
    let n = unpack_normal(px);
    let p = reconstruct_world(px, dims, depth) + n * max(params.min_ray_offset, T_EPS);
    var visible = 0.0;
    for (var s: u32 = 0u; s < sample_count; s = s + 1u) {
        let u1 = hash11(seed_base + s * 2u + 17u);
        let u2 = hash11(seed_base + s * 2u + 53u);
        visible = visible + select(1.0, 0.0, trace_occluder(p, cosine_hemisphere(u1, u2, n), radius));
    }
    return clamp(visible / f32(max(sample_count, 1u)), 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let px = vec2<i32>(id.xy);
    let depth = textureLoad(depth_tex, px, 0).x;
    if (rt_params.enabled == 0u || depth >= 0.9999) {
        textureStore(ao_out, px, vec4<f32>(1.0, 0.0, 0.0, 1.0));
        return;
    }
    let radius = max(0.05, min(bitcast<f32>(rt_params.rtao_radius_bits), params.gi_max_distance));
    let samples = clamp(rt_params.rtao_sample_count, 1u, 16u);
    let seed = id.x * 1973u + id.y * 9277u + u32(max(params.frame_number, 0)) * 26699u;
    var ao = ao_estimate(px, dims, samples, radius, seed);
    let n0 = unpack_normal(px);
    let d0 = depth;
    var weight_sum = 1.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        var tap = vec2<i32>(0, 1);
        if (i == 0u) { tap = vec2<i32>(1, 0); }
        if (i == 1u) { tap = vec2<i32>(-1, 0); }
        if (i == 2u) { tap = vec2<i32>(0, 1); }
        if (i == 3u) { tap = vec2<i32>(0, -1); }
        let q = clamp(px + tap, vec2<i32>(0), vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1));
        let dq = textureLoad(depth_tex, q, 0).x;
        let nq = unpack_normal(q);
        let w = exp(-abs(dq - d0) * 64.0) * max(dot(n0, nq), 0.0);
        if (w > 0.15) {
            let qs = max(samples / 2u, 1u);
            ao = ao + ao_estimate(q, dims, qs, radius, seed + i * 101u + 409u) * w;
            weight_sum = weight_sum + w;
        }
    }
    textureStore(ao_out, px, vec4<f32>(clamp(ao / weight_sum, 0.0, 1.0), 0.0, 0.0, 1.0));
}
