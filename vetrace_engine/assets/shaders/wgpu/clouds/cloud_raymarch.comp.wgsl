// Initial volumetric cloud raymarch prototype.
// The production raytrace shader embeds the same data layout so cloud radiance
// and transmittance can be composited before post-processing.

struct VolumetricCloud {
    center_base_thickness: vec4<f32>,
    coverage_density_noise_phase: vec4<f32>,
    wind_steps: vec4<f32>,
    light_padding: vec4<f32>,
};

struct CloudFrameParams {
    camera_pos_time: vec4<f32>,
    sun_dir_intensity: vec4<f32>,
    sun_color_count: vec4<f32>,
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

fn light_transmittance(cloud: VolumetricCloud, p: vec3<f32>, light_dir: vec3<f32>, jitter: f32) -> f32 {
    let steps = max(1u, min(u32(cloud.light_padding.x), 32u));
    let base_y = cloud.center_base_thickness.y;
    let top_y = base_y + max(cloud.center_base_thickness.w, 0.001);
    let exit_y = select(base_y, top_y, light_dir.y >= 0.0);
    let denom = select(light_dir.y, select(-1e-4, 1e-4, light_dir.y >= 0.0), abs(light_dir.y) < 1e-4);
    let max_dist = max((exit_y - p.y) / denom, 0.0);
    let dt = max_dist / f32(steps);
    var optical_depth = 0.0;
    for (var li: u32 = 0u; li < steps; li = li + 1u) {
        let lp = p + light_dir * ((f32(li) + jitter) * dt);
        optical_depth += density(cloud, lp) * dt;
    }
    let strength = max(cloud.light_padding.y, 0.0);
    return exp(-optical_depth * strength);
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
            let light_t = light_transmittance(cloud, p, normalize(params.sun_dir_intensity.xyz), jitter);
            let scatter = (1.0 - absorb) * transmittance;
            radiance += scatter * light_t * params.sun_color_count.xyz * params.sun_dir_intensity.w;
            transmittance *= absorb;
        }
    }
    textureStore(output_color, vec2<i32>(id.xy), vec4(radiance + base * transmittance, 1.0));
}
