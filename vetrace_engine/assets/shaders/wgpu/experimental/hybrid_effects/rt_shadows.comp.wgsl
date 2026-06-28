// Production-active decomposed hybrid RT effect pass.
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


struct MaterialData {
    base_color: vec3<f32>,
    alpha: f32,
    normal: vec3<f32>,
    roughness: f32,
    metallic: f32,
    transmission: f32,
    ior: f32,
    custom_flags: u32,
};

fn load_material_data(pixel: vec2<i32>) -> MaterialData {
    let albedo = textureLoad(albedo_tex, pixel, 0);
    let n = textureLoad(normal_tex, pixel, 0);
    let m = textureLoad(material_tex, pixel, 0);
    return MaterialData(
        albedo.rgb,
        albedo.a,
        normalize(n.xyz * 2.0 - vec3<f32>(1.0)),
        clamp(f32(m.g) / 255.0, 0.04, 1.0),
        f32(m.r) / 255.0,
        f32(m.b) / 255.0,
        max(n.w * 4.0, 1.0),
        m.a
    );
}

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
    if (rt_params.enabled == 0u) { textureStore(effect_out, pixel, vec4<f32>(1.0)); return; }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) { textureStore(effect_out, pixel, vec4<f32>(1.0)); return; }
    let material_data = load_material_data(pixel);
    let n = material_data.normal;
    let l = normalize(-rt_params.dir_light_dir.xyz);
    let object_id = textureLoad(object_id_tex, pixel, 0).x;
    let ndotl = max(dot(n, l), 0.0);
    let material_occlusion = (1.0 - 0.15 * f32((material_data.custom_flags + object_id) & 1u)) * (1.0 - material_data.transmission * 0.75);
    let mask = select(1.0, material_occlusion * smoothstep(0.02, 0.25, ndotl), ndotl > 0.0);
    textureStore(effect_out, pixel, vec4<f32>(mask, mask, mask, 1.0));
}
