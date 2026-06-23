struct Atmosphere {
    center_radius: vec4<f32>,
    atmo_g_height: vec4<f32>,
    ray_beta: vec4<f32>,
    mie_beta: vec4<f32>,
    ambient_beta: vec4<f32>,
    absorption_beta: vec4<f32>,
    absorb_params: vec4<f32>,
    ozone_params: vec4<f32>,
    multi_scatter_params: vec4<f32>,
};
const MAX_ATMOSPHERES: u32 = 8u;
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
    _pad_atmos: vec2<u32>,
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
fn ozone_density(atmo: Atmosphere, height: f32) -> f32 {
    let center_altitude = atmo.ozone_params.x;
    let thickness = max(atmo.ozone_params.y, 1e-3);
    let strength = max(atmo.ozone_params.z, 0.0);
    let normalized_altitude = (height - center_altitude) / thickness;
    return strength * exp(-normalized_altitude * normalized_altitude);
}
fn integrate_atmosphere(origin: vec3<f32>, dir: vec3<f32>, max_t: f32, multi: vec3<f32>) -> Scattering {
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
    var t = t0 + 0.5 * dt;
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
        let d_a = ozone_density(atmo, h);
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
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(multi_scattering_lut);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    var color = vec3<f32>(0.0);
    if (params.atmosphere != 0u && params.atmo_count > 0u) {
        let atmo = params.atmos[0];
        let u = (f32(id.x) + 0.5) / f32(dims.x);
        let v = (f32(id.y) + 0.5) / f32(dims.y);
        let density = exp(-v * 8.0);
        color = (atmo.ray_beta.xyz * (0.35 + 0.65 * u) + atmo.mie_beta.xyz * (1.0 - u)) * density * 0.2;
    }
    textureStore(multi_scattering_lut, vec2<i32>(id.xy), vec4<f32>(color, 1.0));
}
