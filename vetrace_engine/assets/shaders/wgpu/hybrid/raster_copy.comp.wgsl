@group(0) @binding(0) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(1) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(2) var dst_screen: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var dst_color: texture_storage_2d<rgba16float, write>;

struct LightUniform {
    dir: vec2<f32>;
    _pad: vec2<f32>;
    color: vec3<f32>;
    intensity: f32;
};
@group(0) @binding(4) var<uniform> light: LightUniform;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(dst_screen);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let albedo = textureLoad(gbuf_albedo, vec2<i32>(id.xy), 0).rgb;
    let normal = textureLoad(gbuf_normal, vec2<i32>(id.xy), 0).xyz * 2.0 - 1.0;
    let l = normalize(vec3<f32>(-light.dir, 1.0));
    let diff = max(dot(normal, l), 0.0) * light.intensity;
    let color = albedo * light.color * diff;
    let out_col = vec4<f32>(color, 1.0);
    textureStore(dst_screen, vec2<i32>(id.xy), out_col);
    textureStore(dst_color, vec2<i32>(id.xy), out_col);
}
