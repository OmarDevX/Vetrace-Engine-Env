// Initial volumetric cloud raymarch prototype.
// The production raytrace shader embeds the same data layout so cloud radiance
// and transmittance can be composited before post-processing.

struct VolumetricCloud {
    center_base_thickness: vec4<f32>,
    coverage_density_noise_phase: vec4<f32>,
    wind_steps: vec4<f32>,
    light_padding: vec4<f32>,
    multi_scatter: vec4<f32>,
};

struct GpuAtmosphere {
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

struct CloudFrameParams {
    camera_pos_time: vec4<f32>,
    sun_dir_intensity: vec4<f32>,
    sun_color_count: vec4<f32>,
    planet_shadow: vec4<f32>, // xyz = atmosphere/planet center, w = planet radius
    atmosphere_penumbra: vec4<f32>, // x = atmosphere radius, y = penumbra fallback, z = atmosphere count
};

@group(0) @binding(0) var<storage, read> clouds: array<VolumetricCloud>;
@group(0) @binding(1) var<uniform> params: CloudFrameParams;
@group(0) @binding(2) var input_color: texture_2d<f32>;
@group(0) @binding(3) var output_color: texture_storage_2d<rgba16float, write>;

fn hash31(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
    let d = dot(q, q.yzx + vec3<f32>(33.33));
    return fract((q.x + q.y) * (q.z + d));
}

fn phase_lobe(g: f32, mu: f32) -> f32 {
    let gg = g * g;
    return (1.0 - gg) / max(0.001, pow(1.0 + gg - 2.0 * g * mu, 1.5));
}

fn multi_scatter_lighting(cloud: VolumetricCloud, sigma: f32, mu: f32, light_t: f32) -> vec3<f32> {
    let strength = clamp(cloud.multi_scatter.x, 0.0, 2.0);
    let octaves = min(max(u32(cloud.multi_scatter.y), 0u), 6u);
    let attenuation = clamp(cloud.multi_scatter.z, 0.0, 1.0);
    let eccentricity = clamp(cloud.multi_scatter.w, 0.0, 1.0);
    let density_lift = 1.0 - exp(-max(sigma, 0.0) * 0.75);
    var energy = 0.0;
    var phase_scale = attenuation;
    var g_scale = eccentricity;
    for (var octave: u32 = 0u; octave < octaves; octave = octave + 1u) {
        energy += phase_scale * phase_lobe(clamp(cloud.coverage_density_noise_phase.w * g_scale, -0.95, 0.95), mu);
        phase_scale *= attenuation;
        g_scale *= eccentricity;
    }
    let ambient_energy = density_lift * (0.25 + 0.75 * (1.0 - light_t)) * mix(0.35, 1.0, light_t);
    return params.sun_color_count.xyz * params.sun_dir_intensity.w * clamp(strength * (energy * 0.045 + ambient_energy * 0.18), 0.0, 0.75);
}

fn density(cloud: VolumetricCloud, p: vec3<f32>) -> f32 {
    let h = clamp((p.y - cloud.center_base_thickness.y) / max(cloud.center_base_thickness.w, 0.001), 0.0, 1.0);
    let height_shape = smoothstep(0.0, 0.2, h) * (1.0 - smoothstep(0.75, 1.0, h));
    let wind = vec3<f32>(cloud.wind_steps.x, 0.0, cloud.wind_steps.y) * cloud.wind_steps.z * params.camera_pos_time.w;
    let n = hash31(floor((p + wind) * max(cloud.coverage_density_noise_phase.z, 0.001)));
    return max(0.0, n - (1.0 - cloud.coverage_density_noise_phase.x)) * cloud.coverage_density_noise_phase.y * height_shape;
}

fn blue_noise_jitter(pixel: vec2<u32>, frame: u32) -> f32 {
    let tile = vec2<f32>(pixel & vec2<u32>(127u));
    return hash31(vec3<f32>(tile * vec2<f32>(0.75487766, 0.56984029), f32(frame & 255u) * 0.61803399));
}

fn ray_sphere_forward_interval(ro: vec3<f32>, rd: vec3<f32>, center: vec3<f32>, radius: f32) -> vec2<f32> {
    let oc = ro - center;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - radius * radius;
    let h = b * b - c;
    if (h < 0.0) { return vec2<f32>(1e20, -1e20); }
    let sqrt_h = sqrt(h);
    return vec2<f32>(-b - sqrt_h, -b + sqrt_h);
}

fn planet_shadow(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, max_dist: f32) -> f32 {
    if (params.atmosphere_penumbra.z <= 0.0) { return 1.0; }
    let planet_center = params.planet_shadow.xyz;
    let planet_radius = max(params.planet_shadow.w, 0.0);
    let atmosphere_radius = max(params.atmosphere_penumbra.x, planet_radius);
    let atmosphere_hit = ray_sphere_forward_interval(p, light_dir, planet_center, atmosphere_radius);
    let atmosphere_exit_dist = select(atmosphere_hit.y, 1e20, atmosphere_hit.y <= 0.0);
    let max_shadow_dist = min(max_dist, atmosphere_exit_dist);
    let planet_hit = ray_sphere_forward_interval(p, light_dir, planet_center, planet_radius);
    let hits_planet = planet_hit.y >= 0.0 && max(planet_hit.x, 0.0) <= max_shadow_dist;
    let dist_to_axis = length(cross(p - planet_center, light_dir));
    let penumbra = max(max(cloud.light_padding.z, params.atmosphere_penumbra.y), 1e-3);
    let soft_shadow = smoothstep(planet_radius - penumbra, planet_radius + penumbra, dist_to_axis);
    return select(1.0, soft_shadow, hits_planet);
}

fn light_transmittance(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, jitter: f32) -> f32 {
    let steps = max(1u, min(u32(cloud.light_padding.x), 32u));
    let base_y = cloud.center_base_thickness.y;
    let top_y = base_y + max(cloud.center_base_thickness.w, 0.001);
    let exit_y = select(base_y, top_y, light_dir.y >= 0.0);
    let denom = select(light_dir.y, select(-1e-4, 1e-4, light_dir.y >= 0.0), abs(light_dir.y) < 1e-4);
    let max_dist = max((exit_y - p.y) / denom, 0.0);
    let body_shadow = planet_shadow(cloud, p, light_dir, max_dist);
    if (body_shadow <= 0.0) { return 0.0; }
    let dt = max_dist / f32(steps);
    var optical_depth = 0.0;
    for (var li: u32 = 0u; li < steps; li = li + 1u) {
        let lp = p + light_dir * ((f32(li) + jitter) * dt);
        optical_depth += density(cloud, lp) * dt;
    }
    let strength = max(cloud.light_padding.y, 0.0);
    return exp(-optical_depth * strength) * body_shadow;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(output_color);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let base = textureLoad(input_color, vec2<i32>(id.xy), 0).rgb;
    // Placeholder full-screen prototype. Direction reconstruction will be wired
    // to renderer camera uniforms when clouds graduate to a dedicated pass.
    let rd = normalize(vec3<f32>(0.0, 0.1, 1.0));
    var transmittance = 1.0;
    var radiance = vec3<f32>(0.0);
    for (var ci: u32 = 0u; ci < u32(params.sun_color_count.w); ci = ci + 1u) {
        let cloud = clouds[ci];
        let steps = max(1u, min(u32(cloud.wind_steps.w), 96u));
        for (var si: u32 = 0u; si < steps; si = si + 1u) {
            let p = params.camera_pos_time.xyz + rd * (f32(si) + 0.5);
            let sigma = density(cloud, p);
            let absorb = exp(-sigma);
            let jitter = blue_noise_jitter(id.xy, u32(params.camera_pos_time.w * 60.0) + si);
            let sun_dir = normalize(params.sun_dir_intensity.xyz);
            let light_t = light_transmittance(cloud, p, sun_dir, jitter);
            let mu = clamp(dot(rd, sun_dir), -1.0, 1.0);
            let single_scatter = light_t * params.sun_color_count.xyz * params.sun_dir_intensity.w;
            let multi_scatter = multi_scatter_lighting(cloud, sigma, mu, light_t);
            let scatter = (1.0 - absorb) * transmittance;
            radiance += scatter * min(single_scatter + multi_scatter, params.sun_color_count.xyz * params.sun_dir_intensity.w * 1.25 + vec3<f32>(0.08));
            transmittance *= absorb;
        }
    }
    textureStore(output_color, vec2<i32>(id.xy), vec4(radiance + base * transmittance, 1.0));
}
