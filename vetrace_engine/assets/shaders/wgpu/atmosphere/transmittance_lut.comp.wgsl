struct Atmosphere {
    center_radius: vec4<f32>,
    atmo_g_height: vec4<f32>,
    ray_beta: vec4<f32>,
    mie_beta: vec4<f32>,
    ambient_beta: vec4<f32>,
    absorption_beta: vec4<f32>,
    absorb_params: vec4<f32>,
    ozone_params: vec4<f32>,
    absorption_layer_params: vec4<f32>,
    multi_scatter_params: vec4<f32>,
};
const MAX_ATMOSPHERES: u32 = 8u;
// Must match Rust: vetrace_engine/src/rendering/wgpu_renderer/types.rs::ShaderParams
struct Params {
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_up: vec4<f32>,
    camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>,
    fov: f32,
    num_objects: i32,
    is_fisheye: i32,
    _pad0: i32,
    skycolor: vec4<f32>,
    taa_jitter: vec2<f32>,
    current_time: f32,
    frame_number: i32,
    selected_index: i32,
    max_bounces: i32,
    light_samples: i32,
    dir_shadow_samples: i32,
    shadow_mode: u32,
    raytraced_shadows_enabled: u32,
    shadow_quality: u32,
    max_shadow_rays: u32,
    emissive_shadow_samples: u32,
    directional_shadow_samples: u32,
    cloud_object_shadows_enabled: u32,
    max_rt_shadow_distance: f32,
    rt_shadow_ray_t_max: f32,
    min_soft_shadow_radius: f32,
    raytraced_reflections_enabled: u32,
    _pad_reflections: u32,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    sky_occlusion: f32,
    total_triangles: u32,
    total_bvh_nodes: u32,
    total_tri_bvh_nodes: u32,
    dof_aperture: f32,
    dof_focus_dist: f32,
    dof_enable: u32,
    _pad_dof: u32,
    atmosphere: u32,
    atmo_count: u32,
    cloud_count: u32,
    atmosphere_mode: u32,
    atmosphere_sun_controls: vec4<f32>,
    cloud_history_weight: f32,
    cloud_sample_count: u32,
    cloud_temporal_quality: u32,
    cloud_shadow_mode: u32,
    renderer_mode: u32,
    rt_debug_view: u32,
    rt_debug_counters: u32,
    max_traversal_steps: u32,
    max_transparent_surfaces: u32,
    shadow_max_distance: f32,
    reflection_max_distance: f32,
    gi_max_distance: f32,
    min_ray_offset: f32,
    _pad_atmos: vec3<u32>,
    atmos: array<Atmosphere, MAX_ATMOSPHERES>,
};
struct Scattering { color: vec3<f32>, transmittance: vec3<f32> };
const PI: f32 = 3.14159265359;
fn ray_sphere_intersect(start: vec3<f32>, dir: vec3<f32>, radius: f32) -> vec2<f32> {
    let a = dot(dir, dir);
    let b = 2.0 * dot(dir, start);
    let c = dot(start, start) - radius * radius;
    let d = b * b - 4.0 * a * c;
    if (d < 0.0) { return vec2<f32>(1e9, -1e9); }
    let sqrt_d = sqrt(d);
    return vec2<f32>((-b - sqrt_d) / (2.0 * a), (-b + sqrt_d) / (2.0 * a));
}
fn dir_from_uv(uv: vec2<f32>) -> vec3<f32> {
    let phi = (uv.x - 0.5) * 2.0 * PI;
    let y = uv.y * 2.0 - 1.0;
    let r = sqrt(max(0.0, 1.0 - y * y));
    return normalize(vec3<f32>(sin(phi) * r, y, cos(phi) * r));
}
fn absorption_layer_density(layer: vec4<f32>, altitude: f32) -> f32 {
    let exp_term = layer.x * exp(clamp(layer.y * altitude, -80.0, 80.0));
    let linear_term = layer.z * altitude + layer.w;
    return max(exp_term + linear_term, 0.0);
}

fn absorption_profile_density(atmo: Atmosphere, height: f32) -> f32 {
    let lower_width = max(atmo.absorb_params.x, 0.0);
    let upper_width = max(atmo.absorb_params.y, 0.0);
    let density_scale = max(atmo.absorb_params.z, 0.0);
    let lower_density = select(0.0, absorption_layer_density(atmo.ozone_params, height), height <= lower_width);
    let upper_altitude = max(height - lower_width, 0.0);
    let upper_density = select(0.0, absorption_layer_density(atmo.absorption_layer_params, height), height > lower_width && upper_altitude <= upper_width);
    return density_scale * clamp(max(lower_density, upper_density), 0.0, 1.0);
}


fn transmittance_lut_uv(atmo: Atmosphere, origin: vec3<f32>, view_dir: vec3<f32>) -> vec2<f32> {
    let up = normalize(origin - atmo.center_radius.xyz);
    let height = clamp(length(origin - atmo.center_radius.xyz) - atmo.center_radius.w, 0.0, max(atmo.atmo_g_height.x - atmo.center_radius.w, 1e-3));
    let height_u = height / max(atmo.atmo_g_height.x - atmo.center_radius.w, 1e-3);
    let zenith_u = dot(normalize(view_dir), up) * 0.5 + 0.5;
    return clamp(vec2<f32>(zenith_u, height_u), vec2<f32>(0.0), vec2<f32>(1.0));
}

fn integrate_transmittance(atmo: Atmosphere, origin: vec3<f32>, dir: vec3<f32>) -> vec3<f32> {
    let pos_rel = origin - atmo.center_radius.xyz;
    let seg = ray_sphere_intersect(pos_rel, dir, atmo.atmo_g_height.x);
    var t0 = max(seg.x, 0.0);
    var t1 = max(seg.y, 0.0);
    if (t0 > t1) { return vec3<f32>(1.0); }
    let steps = 48;
    let dt = (t1 - t0) / f32(steps);
    var t = t0 + 0.5 * dt;
    let inv_hr = 1.0 / max(atmo.atmo_g_height.z, 1e-3);
    let inv_hm = 1.0 / max(atmo.atmo_g_height.w, 1e-3);
    var tau_r = 0.0;
    var tau_m = 0.0;
    var tau_a = 0.0;
    for (var i = 0; i < steps; i = i + 1) {
        let sp = pos_rel + dir * t;
        let h = max(0.0, length(sp) - atmo.center_radius.w);
        tau_r += exp(-h * inv_hr) * dt;
        tau_m += exp(-h * inv_hm) * dt;
        tau_a += absorption_profile_density(atmo, h) * dt;
        t += dt;
    }
    return exp(-(atmo.ray_beta.xyz * vec3<f32>(tau_r) + atmo.mie_beta.xyz * vec3<f32>(tau_m) + atmo.absorption_beta.xyz * vec3<f32>(tau_a)));
}

@group(0) @binding(0) var transmittance_lut: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var unused_lut: texture_2d<f32>;
@group(0) @binding(3) var blue_noise_tex: texture_2d<f32>;
@group(0) @binding(4) var blue_noise_sampler: sampler;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(transmittance_lut);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    var trans = vec3<f32>(1.0);
    if (params.atmosphere != 0u && params.atmo_count > 0u) {
        let atmo = params.atmos[0];
        let height_u = (f32(id.y) + 0.5) / f32(dims.y);
        let zenith_cos = ((f32(id.x) + 0.5) / f32(dims.x)) * 2.0 - 1.0;
        let up = vec3<f32>(0.0, 1.0, 0.0);
        let tangent = vec3<f32>(1.0, 0.0, 0.0);
        let height = height_u * max(atmo.atmo_g_height.x - atmo.center_radius.w, 1e-3);
        let origin = atmo.center_radius.xyz + up * (atmo.center_radius.w + height);
        let dir = normalize(up * zenith_cos + tangent * sqrt(max(0.0, 1.0 - zenith_cos * zenith_cos)));
        trans = integrate_transmittance(atmo, origin, dir);
    }
    textureStore(transmittance_lut, vec2<i32>(id.xy), vec4<f32>(trans, 1.0));
}
