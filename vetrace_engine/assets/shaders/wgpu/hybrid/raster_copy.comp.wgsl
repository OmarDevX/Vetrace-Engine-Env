@group(0) @binding(0) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(1) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(2) var dst_screen: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var dst_color: texture_storage_2d<rgba16float, write>;

struct LightUniform {
    dir: vec2<f32>,
    _pad: vec2<f32>,
    color: vec3<f32>,
    intensity: f32,
};
@group(0) @binding(4) var<uniform> light: LightUniform;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(dst_screen);
    if (id.x >= dims.x || id.y >= dims.y) { return; }

    let uv = vec2<i32>(id.xy);
    let albedo = textureLoad(gbuf_albedo, uv, 0);
    let normal = normalize(textureLoad(gbuf_normal, uv, 0).xyz * 2.0 - 1.0);
    let ldir = normalize(vec3<f32>(light.dir, -1.0));
    let diff = max(dot(normal, -ldir), 0.0);
    let lit = vec4<f32>(albedo.rgb * light.color * diff * light.intensity, albedo.a);

    textureStore(dst_screen, uv, lit);
    textureStore(dst_color, uv, lit);
}

