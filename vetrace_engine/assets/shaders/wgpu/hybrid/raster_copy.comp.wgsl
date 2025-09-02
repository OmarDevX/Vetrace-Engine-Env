@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var dst_screen: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var dst_color: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(dst_screen);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let texel = textureLoad(src, vec2<i32>(id.xy), 0);
    let out_col = vec4<f32>(texel.rgb, 1.0);
    textureStore(dst_screen, vec2<i32>(id.xy), out_col);
    textureStore(dst_color, vec2<i32>(id.xy), out_col);
}
