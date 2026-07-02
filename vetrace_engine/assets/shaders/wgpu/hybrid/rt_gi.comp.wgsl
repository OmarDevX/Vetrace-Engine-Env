// Production-active decomposed hybrid one-bounce RTGI effect pass.
const GI_MODE_RTGI_ONE_BOUNCE: u32 = 4u;
const T_EPS: f32 = 0.002;
const INF_T: f32 = 1.0e20;
const PI: f32 = 3.14159265359;

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
@group(0) @binding(6) var effect_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;
@group(0) @binding(21) var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(22) var material_sampler: sampler;
// Shared BVH declarations/traversal are concatenated by Rust from hybrid/bvh_traversal.wgsl.

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip_xy = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var world = rt_params.inv_view_proj * vec4<f32>(clip_xy, depth, 1.0);
    return (world / max(world.w, 1.0e-6)).xyz;
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

fn cosine_dir(n: vec3<f32>, pixel: vec2<u32>, sample: u32) -> vec3<f32> {
    let seed = pixel.x * 1973u + pixel.y * 9277u + u32(max(params.frame_number, 0)) * 26699u + sample * 101u;
    let u1 = hash11(seed + 17u);
    let u2 = hash11(seed + 53u);
    let r = sqrt(u1);
    let phi = 2.0 * PI * u2;
    let local = vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - u1)));
    return normalize(tangent_basis(n) * local);
}

fn visible_to_light(pos: vec3<f32>, n: vec3<f32>, l: vec3<f32>, max_objects: u32) -> f32 {
    if (dot(n, l) <= 0.0) { return 0.0; }
    let h = trace_scene(pos + n * T_EPS, l, max_objects, 64u);
    return select(1.0, 0.0, h.hit != 0u && h.t < min(params.max_rt_shadow_distance, params.rt_shadow_ray_t_max));
}
fn sky_radiance(rd: vec3<f32>) -> vec3<f32> {
    let horizon = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    return params.skycolor.rgb * (0.35 + 0.65 * horizon) * max(0.0, 1.0 - params.sky_occlusion);
}
fn material_radiance(hit: Hit, hit_pos: vec3<f32>, max_objects: u32) -> vec3<f32> {
    let mat = materials[hit.material_index];
    var albedo = mat.baseColorFactor.rgb;
    if (mat.baseColorTex != 0u) {
        albedo = mat.baseColorFactor.rgb * textureSampleLevel(textures[mat.baseColorTex], material_sampler, hit.uv, 0.0).rgb;
    }
    var emissive_texel = vec3<f32>(1.0);
    let emissive_tex = mat.material_flags1;
    if (emissive_tex != 0u) {
        emissive_texel = textureSampleLevel(textures[emissive_tex], material_sampler, hit.uv, 0.0).rgb;
    }
    let emissive = mat.emissiveFactor * mat.emissiveStrength * emissive_texel;
    let l = normalize(-params.dir_light_dir.xyz);
    let ndotl = max(dot(hit.normal, l), 0.0);
    let vis = visible_to_light(hit_pos + hit.normal * T_EPS, hit.normal, l, max_objects);
    let direct = params.dir_light_color.rgb * max(params.dir_light_dir.w, 0.0) * ndotl * vis;
    return emissive + albedo * direct;
}
fn write_miss(pixel: vec2<i32>) { textureStore(effect_out, pixel, vec4<f32>(0.0)); }

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (rt_params.enabled == 0u || rt_params.gi_mode != GI_MODE_RTGI_ONE_BOUNCE) { write_miss(pixel); return; }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) { write_miss(pixel); return; }
    let n = unpack_normal(pixel);
    let world = reconstruct_world(pixel, dims, depth);
    // GI output is incident irradiance. The final PBR compose applies the
    // receiver albedo. Multiplying by surface_albedo here made RTGI double-tint
    // and often appear almost black on colored/dark materials.
    // Hybrid RTGI is used as a real-time indirect-light hint, not an offline
    // path tracer. Keep it 1 spp and let gi_resolve do the spatial/temporal
    // filtering; this removes most close-up stalls while preserving color bleed.
    let adaptive_samples = u32(max(params.light_samples, 1));
    let high_quality = adaptive_samples >= 2u && params.max_bounces > 1;
    let rays = 1u;
    let max_objects = select(128u, 320u, high_quality);
    let max_tris = select(160u, 512u, high_quality);
    var sum = vec3<f32>(0.0);
    for (var s = 0u; s < rays; s = s + 1u) {
        let rd = cosine_dir(n, id.xy, s);
        let hit = trace_scene(world + n * max(params.min_ray_offset, T_EPS), rd, max_objects, max_tris);
        var incoming = sky_radiance(rd);
        if (hit.hit != 0u) {
            incoming = material_radiance(hit, hit.pos, max_objects);
        }
        sum = sum + incoming;
    }
    let irradiance = sum / f32(rays);
    textureStore(effect_out, pixel, vec4<f32>(max(irradiance, vec3<f32>(0.0)), 1.0));
}
