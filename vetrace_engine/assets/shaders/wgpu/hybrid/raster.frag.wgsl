struct VsOut {
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(9) var gbuf_albedo: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(10) var gbuf_normal: texture_storage_2d<rgba16float, write>;
@group(0) @binding(11) var gbuf_material: texture_storage_2d<rgba8uint, write>;

@fragment
fn main(in: VsOut) {
    let uv = in.uv;
    textureStore(gbuf_albedo, vec2<i32>(i32(uv.x), i32(uv.y)), vec4<f32>(1.0, 0.0, 0.0, 1.0));
    textureStore(gbuf_normal, vec2<i32>(i32(uv.x), i32(uv.y)), vec4<f32>(0.0, 0.0, 1.0, 1.0));
    textureStore(gbuf_material, vec2<i32>(i32(uv.x), i32(uv.y)), vec4<u32>(0u,0u,0u,0u));
}
