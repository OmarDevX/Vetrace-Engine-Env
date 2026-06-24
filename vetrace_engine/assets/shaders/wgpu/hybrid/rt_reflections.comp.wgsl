struct RtEffectParams {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    enabled: u32,
    mode: u32,
    _pad: vec2<u32>,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<f32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var effect_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    var world = rt_params.inv_view_proj * clip;
    world = world / world.w;
    return world.xyz;
}

fn miss(pixel: vec2<i32>) {
    textureStore(effect_out, pixel, vec4<f32>(0.0, 0.0, 0.0, 0.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (rt_params.enabled == 0u) { miss(pixel); return; }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) { miss(pixel); return; }
    let world = reconstruct_world(pixel, dims, depth);
    let n = unpack_normal(pixel);
    let albedo = textureLoad(albedo_tex, pixel, 0).rgb;
    let roughness = clamp(textureLoad(roughness_tex, pixel, 0).x, 0.04, 1.0);
    let v = normalize(rt_params.camera_pos.xyz - world);
    let f0 = mix(vec3<f32>(0.04), albedo, f32(textureLoad(material_tex, pixel, 0).r) / 255.0);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    let reflection = mix(rt_params.dir_light_color.rgb, albedo, roughness) * fresnel * (1.0 - roughness);
    textureStore(effect_out, pixel, vec4<f32>(reflection, 1.0 - roughness));
}
