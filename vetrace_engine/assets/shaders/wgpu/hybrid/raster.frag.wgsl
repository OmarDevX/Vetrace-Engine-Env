struct VsOut {
    @location(0) uv: vec2<f32>,
};

struct MaterialOutput {
    base_color: vec4<f32>,
    normal: vec3<f32>,
    roughness: f32,
    metallic: f32,
    emissive: vec3<f32>,
    alpha: f32,
    transmission: f32,
    ior: f32,
    custom_flags: u32,
};

const MATERIAL_FLAG_RASTER_ONLY: u32 = 1u;
const MATERIAL_FLAG_FALLBACK_TO_RASTER_DATA: u32 = 2u;

@group(0) @binding(9) var gbuf_albedo: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(10) var gbuf_normal: texture_storage_2d<rgba16float, write>;
@group(0) @binding(11) var gbuf_material: texture_storage_2d<rgba8uint, write>;

fn default_material_output() -> MaterialOutput {
    return MaterialOutput(
        vec4<f32>(1.0, 0.0, 0.0, 1.0),
        vec3<f32>(0.0, 0.0, 1.0),
        0.5,
        0.0,
        vec3<f32>(0.0),
        1.0,
        0.0,
        1.5,
        0u,
    );
}

fn encode_material_byte(v: f32) -> u32 {
    return u32(round(clamp(v, 0.0, 1.0) * 255.0));
}

@fragment
fn main(in: VsOut) {
    let uv = in.uv;
    let pixel = vec2<i32>(i32(uv.x), i32(uv.y));
    let material = default_material_output();

    // Shared GBuffer contract for raster primary and RT effects:
    // albedo.rgb = base color, albedo.a = alpha/transparency coverage
    // normal.xyz = encoded normal, normal.w = IOR / 4.0
    // material.r = metallic, material.g = roughness, material.b = transmission, material.a = custom flags
    textureStore(gbuf_albedo, pixel, vec4<f32>(material.base_color.rgb + material.emissive, material.alpha));
    textureStore(gbuf_normal, pixel, vec4<f32>(normalize(material.normal) * 0.5 + vec3<f32>(0.5), clamp(material.ior / 4.0, 0.0, 1.0)));
    textureStore(gbuf_material, pixel, vec4<u32>(
        encode_material_byte(material.metallic),
        encode_material_byte(material.roughness),
        encode_material_byte(material.transmission),
        material.custom_flags & 255u,
    ));
}
