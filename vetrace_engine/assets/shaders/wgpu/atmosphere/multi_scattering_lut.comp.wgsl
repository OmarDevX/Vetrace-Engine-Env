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


fn transmittance_lut_uv(atmo: Atmosphere, origin: vec3<f32>, light_dir: vec3<f32>) -> vec2<f32> {
    let rel = origin - atmo.center_radius.xyz;
    let r = clamp((length(rel) - atmo.center_radius.w) / max(atmo.atmo_g_height.x - atmo.center_radius.w, 1e-3), 0.0, 1.0);
    let up = normalize(select(vec3<f32>(0.0, 1.0, 0.0), rel, length(rel) > 1e-3));
    let mu = dot(up, light_dir) * 0.5 + 0.5;
    return vec2<f32>(clamp(mu, 0.0, 1.0), r);
}

fn sample_transmittance_lut(atmo: Atmosphere, origin: vec3<f32>, light_dir: vec3<f32>) -> vec3<f32> {
    return textureSampleLevel(transmittance_lut, blue_noise_sampler, transmittance_lut_uv(atmo, origin, light_dir), 0.0).xyz;
}

fn sample_blue_noise(pixel: vec2<u32>, frame_number: i32) -> f32 {
    let dims = vec2<u32>(textureDimensions(blue_noise_tex, 0));
    let frame = u32(max(frame_number, 0));
    let offset = vec2<u32>((frame * 5u + frame / 3u) % dims.x, (frame * 7u + frame / 5u) % dims.y);
    let coord = (pixel + offset) % dims;
    return textureSampleLevel(blue_noise_tex, blue_noise_sampler, (vec2<f32>(coord) + vec2<f32>(0.5)) / vec2<f32>(dims), 0.0).r;
}

fn integrate_atmosphere(origin: vec3<f32>, dir: vec3<f32>, max_t: f32, multi: vec3<f32>, pixel: vec2<u32>, frame_number: i32) -> Scattering {
    if (params.atmosphere == 0u || params.atmo_count == 0u) {
        return Scattering(vec3<f32>(0.0), vec3<f32>(1.0));
    }
    let atmo = params.atmos[0];
    let sun_dir = normalize(-params.dir_light_dir.xyz);
    let sun_i = params.dir_light_color.xyz * params.dir_light_dir.w;
    let center = atmo.center_radius.xyz;
    let pos_rel = origin - center;
    let seg = ray_sphere_intersect(pos_rel, dir, atmo.atmo_g_height.x);
    var t0 = max(seg.x, 0.0);
    var t1 = min(seg.y, max_t);
    if (t0 > t1) { return Scattering(vec3<f32>(0.0), vec3<f32>(1.0)); }
    let steps = 18;
    let dt = (t1 - t0) / f32(steps);
    var t = t0 + (0.25 + 0.5 * sample_blue_noise(pixel, frame_number)) * dt;
    let inv_hr = 1.0 / max(atmo.atmo_g_height.z, 1e-3);
    let inv_hm = 1.0 / max(atmo.atmo_g_height.w, 1e-3);
    let mu = dot(dir, sun_dir);
    let mumu = mu * mu;
    let g = atmo.atmo_g_height.y;
    let gg = g * g;
    let phase_r = 0.05968310366 * (1.0 + mumu);
    let den = max(1e-3, 1.0 + gg - 2.0 * mu * g);
    let phase_m = 0.11936620732 * (1.0 - gg) * (1.0 + mumu) / (den * sqrt(den));
    var tau_r = 0.0;
    var tau_m = 0.0;
    var tau_a = 0.0;
    var acc_r = vec3<f32>(0.0);
    var acc_m = vec3<f32>(0.0);
    for (var i = 0; i < steps; i = i + 1) {
        let sp = pos_rel + dir * t;
        let h = max(0.0, length(sp) - atmo.center_radius.w);
        let d_r = exp(-h * inv_hr);
        let d_m = exp(-h * inv_hm);
        let d_a = absorption_profile_density(atmo, h);
        tau_r += d_r * dt;
        tau_m += d_m * dt;
        tau_a += d_a * dt;
        let tau = atmo.ray_beta.xyz * vec3<f32>(tau_r) + atmo.mie_beta.xyz * vec3<f32>(tau_m) + atmo.absorption_beta.xyz * vec3<f32>(tau_a);
        let view_t = exp(-tau);
        let light_path = ray_sphere_intersect(sp, sun_dir, atmo.atmo_g_height.x).y;
        let light_tau = max(light_path, 0.0) * 0.00004;
        let light_t = exp(-(atmo.ray_beta.xyz + atmo.mie_beta.xyz + atmo.absorption_beta.xyz) * vec3<f32>(light_tau));
        acc_r += d_r * view_t * light_t * dt;
        acc_m += d_m * view_t * light_t * dt;
        t += dt;
    }
    let color = (phase_r * atmo.ray_beta.xyz * acc_r + phase_m * atmo.mie_beta.xyz * acc_m + tau_r * (atmo.ambient_beta.xyz + multi)) * sun_i;
    let trans = exp(-(atmo.ray_beta.xyz * vec3<f32>(tau_r) + atmo.mie_beta.xyz * vec3<f32>(tau_m) + atmo.absorption_beta.xyz * vec3<f32>(tau_a)));
    return Scattering(color, trans);
}

@group(0) @binding(0) var multi_scattering_lut: texture_storage_2d<rgba16float, write>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var unused_lut: texture_2d<f32>;
@group(0) @binding(3) var blue_noise_tex: texture_2d<f32>;
@group(0) @binding(4) var blue_noise_sampler: sampler;
@group(0) @binding(5) var transmittance_lut: texture_2d<f32>;
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(multi_scattering_lut);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    var color = vec3<f32>(0.0);
    if (params.atmosphere != 0u && params.atmo_count > 0u) {
        let atmo = params.atmos[0];
        // X axis: cosine of the angle between the lookup view ray and the sun direction.
        // Y axis: normalized camera/sample altitude from ground radius to atmosphere radius.
        let view_sun_cos = ((f32(id.x) + 0.5) / f32(dims.x)) * 2.0 - 1.0;
        let altitude_u = (f32(id.y) + 0.5) / f32(dims.y);
        let atmosphere_height = max(atmo.atmo_g_height.x - atmo.center_radius.w, 1e-3);
        let altitude = altitude_u * atmosphere_height;
        let density_r = exp(-altitude / max(atmo.atmo_g_height.z, 1e-3));
        let density_m = exp(-altitude / max(atmo.atmo_g_height.w, 1e-3));
        let sun_dir = normalize(-params.dir_light_dir.xyz);
        let up = vec3<f32>(sqrt(max(0.0, 1.0 - view_sun_cos * view_sun_cos)), view_sun_cos, 0.0);
        let origin = atmo.center_radius.xyz + up * (atmo.center_radius.w + altitude);
        let trans_to_sun = sample_transmittance_lut(atmo, origin, sun_dir);
        let mumu = view_sun_cos * view_sun_cos;
        let g = atmo.atmo_g_height.y;
        let gg = g * g;
        let phase_r = 0.05968310366 * (1.0 + mumu);
        let den = max(1e-3, 1.0 + gg - 2.0 * view_sun_cos * g);
        let phase_m = 0.11936620732 * (1.0 - gg) * (1.0 + mumu) / (den * sqrt(den));
        color = (atmo.ray_beta.xyz * density_r * phase_r + atmo.mie_beta.xyz * density_m * phase_m) * trans_to_sun * atmo.multi_scatter_params.x * max(params.atmosphere_sun_controls.z, 0.0);
    }
    textureStore(multi_scattering_lut, vec2<i32>(id.xy), vec4<f32>(color, 1.0));
}
