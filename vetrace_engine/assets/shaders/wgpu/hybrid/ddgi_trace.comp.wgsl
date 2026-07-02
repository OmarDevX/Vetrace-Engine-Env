// DDGI probe ray tracing pass. Intended to be concatenated after
// pbr_lighting.wgsl and bvh_traversal.wgsl by the renderer.
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

struct DdgiParams {
    probe_counts: vec4<u32>,        // xyz counts, w total probes
    volume_min: vec4<f32>,         // xyz origin
    volume_spacing: vec4<f32>,     // xyz spacing
    atlas_size: vec4<u32>,         // xy irradiance, zw distance atlas sizes
    rays_per_probe: u32,
    probe_inner_size: u32,
    distance_inner_size: u32,
    enabled: u32,
    normal_bias: f32,
    view_bias: f32,
    hysteresis: f32,
    max_ray_distance: f32,
    camera_pos: vec4<f32>,
};

@group(0) @binding(8) var<uniform> params: Params;
@group(0) @binding(21) var textures: binding_array<texture_2d<f32>>;
@group(0) @binding(22) var material_sampler: sampler;
@group(1) @binding(0) var<uniform> ddgi: DdgiParams;
@group(1) @binding(1) var<storage, read> probe_offsets: array<vec4<f32>>;
@group(1) @binding(2) var<storage, read> probe_states: array<u32>;
@group(1) @binding(3) var ray_radiance_out: texture_storage_2d<rgba16float, write>;
@group(1) @binding(4) var ray_distance_out: texture_storage_2d<rg16float, write>;

fn probe_index_to_grid(index: u32) -> vec3<u32> {
    let nx = max(ddgi.probe_counts.x, 1u);
    let ny = max(ddgi.probe_counts.y, 1u);
    return vec3<u32>(index % nx, (index / nx) % ny, index / max(nx * ny, 1u));
}
fn probe_world_position(index: u32) -> vec3<f32> {
    let g = vec3<f32>(probe_index_to_grid(index));
    return ddgi.volume_min.xyz + g * ddgi.volume_spacing.xyz + probe_offsets[index].xyz;
}
fn radical_inverse_vdc(bits_in: u32) -> f32 {
    var bits = bits_in;
    bits = (bits << 16u) | (bits >> 16u);
    bits = ((bits & 0x55555555u) << 1u) | ((bits & 0xAAAAAAAAu) >> 1u);
    bits = ((bits & 0x33333333u) << 2u) | ((bits & 0xCCCCCCCCu) >> 2u);
    bits = ((bits & 0x0F0F0F0Fu) << 4u) | ((bits & 0xF0F0F0F0u) >> 4u);
    bits = ((bits & 0x00FF00FFu) << 8u) | ((bits & 0xFF00FF00u) >> 8u);
    return f32(bits) * 2.3283064365386963e-10;
}
fn spherical_fibonacci(i: u32, n: u32) -> vec3<f32> {
    let count = max(f32(n), 1.0);
    let phi = 2.39996322972865332 * f32(i);
    let z = 1.0 - (2.0 * (f32(i) + 0.5) / count);
    let r = sqrt(max(0.0, 1.0 - z * z));
    return vec3<f32>(cos(phi) * r, sin(phi) * r, z);
}
fn visible_to_light(pos: vec3<f32>, n: vec3<f32>, l: vec3<f32>) -> f32 {
    if (dot(n, l) <= 0.0) { return 0.0; }
    let h = trace_scene_limit(pos + n * max(params.min_ray_offset, T_EPS), l, min(params.max_rt_shadow_distance, params.rt_shadow_ray_t_max));
    return select(1.0, 0.0, h.hit != 0u);
}
fn sky_radiance(rd: vec3<f32>) -> vec3<f32> {
    let horizon = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    return params.skycolor.rgb * (0.35 + 0.65 * horizon) * max(0.0, 1.0 - params.sky_occlusion);
}
fn surface_radiance(hit: Hit, view_dir: vec3<f32>) -> vec3<f32> {
    let mat = materials[hit.material_index];
    var albedo = mat.baseColorFactor.rgb;
    if (mat.baseColorTex != 0u) {
        albedo = mat.baseColorFactor.rgb * textureSampleLevel(textures[mat.baseColorTex], material_sampler, hit.uv, 0.0).rgb;
    }
    let l = normalize(-params.dir_light_dir.xyz);
    let direct = pbr_direct_light(PbrDirectLightInput(albedo, hit.normal, view_dir, l, params.dir_light_color.rgb * max(params.dir_light_dir.w, 0.0), mat.metallicFactor, mat.roughnessFactor, visible_to_light(hit.pos, hit.normal, l)));
    var emissive_texel = vec3<f32>(1.0);
    if (mat.material_flags1 != 0u) { emissive_texel = textureSampleLevel(textures[mat.material_flags1], material_sampler, hit.uv, 0.0).rgb; }
    return max(direct + mat.emissiveFactor * mat.emissiveStrength * emissive_texel, vec3<f32>(0.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (ddgi.enabled == 0u || id.x >= ddgi.rays_per_probe || id.y >= ddgi.probe_counts.w) { return; }
    if (probe_states[id.y] == 0u) {
        textureStore(ray_radiance_out, vec2<i32>(id.xy), vec4<f32>(0.0));
        textureStore(ray_distance_out, vec2<i32>(id.xy), vec4<f32>(ddgi.max_ray_distance, ddgi.max_ray_distance * ddgi.max_ray_distance, 0.0, 0.0));
        return;
    }
    let rd = normalize(spherical_fibonacci(id.x + u32(max(params.frame_number, 0)) * 17u, ddgi.rays_per_probe));
    let origin = probe_world_position(id.y);
    let hit = trace_scene_limit(origin + rd * ddgi.view_bias, rd, ddgi.max_ray_distance);
    var radiance = sky_radiance(rd);
    var dist = ddgi.max_ray_distance;
    if (hit.hit != 0u) {
        dist = hit.t;
        radiance = surface_radiance(hit, rd);
    }
    textureStore(ray_radiance_out, vec2<i32>(id.xy), vec4<f32>(radiance, 1.0));
    textureStore(ray_distance_out, vec2<i32>(id.xy), vec4<f32>(dist, dist * dist, 0.0, 0.0));
}
