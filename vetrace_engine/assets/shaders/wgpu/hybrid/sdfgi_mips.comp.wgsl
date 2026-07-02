// Builds the mip chain for the SDF volume using a simple box filter
@group(0) @binding(0) var src_tex: texture_3d<f32>;
@group(0) @binding(1) var dst_tex: texture_storage_3d<r32float, write>;

@compute @workgroup_size(8,8,4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(dst_tex);
    if (id.x >= dims.x || id.y >= dims.y || id.z >= dims.z) { return; }
    let base = id * 2u;
    var sum = 0.0;
    for (var z: u32 = 0u; z < 2u; z = z + 1u) {
        for (var y: u32 = 0u; y < 2u; y = y + 1u) {
            for (var x: u32 = 0u; x < 2u; x = x + 1u) {
                sum += textureLoad(src_tex, vec3<i32>(base + vec3<u32>(x, y, z)), 0).x;
            }
        }
    }
    textureStore(dst_tex, vec3<i32>(id), vec4(sum / 8.0, 0.0, 0.0, 0.0));
}
