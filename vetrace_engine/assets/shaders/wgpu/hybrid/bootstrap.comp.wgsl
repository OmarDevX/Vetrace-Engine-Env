@group(0) @binding(5)
var color_tex: texture_storage_2d<rgba16float, write>;

@group(0) @binding(37)
var cloud_shadow_optical_depth_tex: texture_storage_2d<r16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(color_tex);

    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let uv = vec2<f32>(
        f32(id.x) / max(f32(dims.x), 1.0),
        f32(id.y) / max(f32(dims.y), 1.0)
    );

    let color = vec4<f32>(
        0.08 + 0.25 * uv.x,
        0.08 + 0.18 * uv.y,
        0.18 + 0.20 * (1.0 - uv.y),
        1.0
    );

    textureStore(color_tex, vec2<i32>(id.xy), color);
}

@compute @workgroup_size(8, 8, 1)
fn cloud_shadow_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(cloud_shadow_optical_depth_tex);

    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    textureStore(
        cloud_shadow_optical_depth_tex,
        vec2<i32>(id.xy),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
}
