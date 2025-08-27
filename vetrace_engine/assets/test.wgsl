fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let h = hsv.x * 6.0;
    let s = hsv.y;
    let v = hsv.z;
    let c = v * s;
    let x = c * (1.0 - abs(fract(h * 0.5) * 2.0 - 1.0));
    let m = v - c;
    var rgb = vec3<f32>(0.0);
    if (h < 1.0) { rgb = vec3<f32>(c, x, 0.0); }
    else if (h < 2.0) { rgb = vec3<f32>(x, c, 0.0); }
    else if (h < 3.0) { rgb = vec3<f32>(0.0, c, x); }
    else if (h < 4.0) { rgb = vec3<f32>(0.0, x, c); }
    else if (h < 5.0) { rgb = vec3<f32>(x, 0.0, c); }
    else { rgb = vec3<f32>(c, 0.0, x); }
    return rgb + m;
}

fn evaluate_rainbow_material(
    hit_point: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    uv: vec2<f32>,
    params: CustomMaterialParams
) -> MaterialResult {
    var result: MaterialResult;
    let rainbow_factor = hit_point.x;
    let hue = fract(rainbow_factor);
    let rainbow_color = hsv_to_rgb(vec3<f32>(hue, 1.0, 1.0));
    result.base_color = rainbow_color * params.color_tint.rgb;
    result.normal = normal;
    result.roughness = params.roughness;
    result.metallic = params.metallic;
    result.emission = rainbow_color * params.emission_strength;
    result.transparency = params.transparency;
    result.transmission = params.transmission;
    result.transmission_roughness = params.transmission_roughness;
    result.ior = params.refraction_ior;
    return result;
}