const PBR_PI: f32 = 3.141592653589793;

struct PbrDirectLightInput {
    albedo: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    light_radiance: vec3<f32>,
    metallic: f32,
    roughness: f32,
    visibility: f32,
};

fn pbr_saturate(v: f32) -> f32 {
    return clamp(v, 0.0, 1.0);
}

fn pbr_f0(albedo: vec3<f32>, metallic: f32) -> vec3<f32> {
    return mix(vec3<f32>(0.04), albedo, pbr_saturate(metallic));
}

fn pbr_fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    let f = pow(1.0 - pbr_saturate(cos_theta), 5.0);
    return f0 + (vec3<f32>(1.0) - f0) * f;
}

fn pbr_distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let r = max(roughness, 0.04);
    let a = r * r;
    let a2 = a * a;
    let nh = pbr_saturate(n_dot_h);
    let denom = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / max(PBR_PI * denom * denom, 1e-5);
}

fn pbr_geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = max(roughness, 0.04) + 1.0;
    let k = (r * r) / 8.0;
    let nv = pbr_saturate(n_dot_v);
    return nv / max(nv * (1.0 - k) + k, 1e-5);
}

fn pbr_geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return pbr_geometry_schlick_ggx(n_dot_v, roughness) * pbr_geometry_schlick_ggx(n_dot_l, roughness);
}

fn pbr_direct_light(input: PbrDirectLightInput) -> vec3<f32> {
    let n = normalize(input.normal);
    let v = normalize(-input.view_dir);
    let l = normalize(input.light_dir);
    let h = normalize(v + l);
    let n_dot_l = pbr_saturate(dot(n, l));
    let n_dot_v = max(pbr_saturate(dot(n, v)), 1e-4);
    let n_dot_h = pbr_saturate(dot(n, h));
    let v_dot_h = pbr_saturate(dot(v, h));
    let metallic = pbr_saturate(input.metallic);
    let roughness = clamp(input.roughness, 0.04, 1.0);
    let f0 = pbr_f0(input.albedo, metallic);
    let f = pbr_fresnel_schlick(v_dot_h, f0);
    let d = pbr_distribution_ggx(n_dot_h, roughness);
    let g = pbr_geometry_smith(n_dot_v, n_dot_l, roughness);
    let specular = (d * g * f) / max(4.0 * n_dot_v * n_dot_l, 1e-5);
    let diffuse = input.albedo * (vec3<f32>(1.0) - f) * (1.0 - metallic) / PBR_PI;
    return (diffuse + specular) * input.light_radiance * n_dot_l * pbr_saturate(input.visibility);
}

fn pbr_ambient_diffuse(albedo: vec3<f32>, irradiance: vec3<f32>, metallic: f32) -> vec3<f32> {
    return albedo * irradiance * (1.0 - pbr_saturate(metallic));
}

fn pbr_reflection_fresnel(albedo: vec3<f32>, normal: vec3<f32>, view_dir: vec3<f32>, metallic: f32) -> vec3<f32> {
    let v = normalize(-view_dir);
    return pbr_fresnel_schlick(max(dot(normalize(normal), v), 0.0), pbr_f0(albedo, metallic));
}
