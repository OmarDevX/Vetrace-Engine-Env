// Screen space outline pass

struct Params {
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_up: vec4<f32>,
    camera_right: vec4<f32>,
    fov: f32,
    num_objects: i32,
    is_fisheye: i32,
    _pad0: i32,
    skycolor: vec4<f32>,
    taa_jitter: vec2<f32>,
    current_time: f32,
    frame_number: i32,
    selected_index: i32,
    _prepad: vec4<i32>,
    _pad1: vec4<i32>,
    _pad2: i32,
    _pad_end: vec2<i32>
};

@group(0) @binding(0)
var color_tex: texture_2d<f32>;
@group(0) @binding(1)
var<uniform> params: Params;
@group(0) @binding(2)
var out_tex: texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(out_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let sample = textureLoad(color_tex, vec2<i32>(id.xy), 0);
    var color = sample.xyz;
    let obj_idx = i32(round(sample.w));
   let mask = u32(params.selected_index);
    if (((mask >> u32(obj_idx)) & 1u) == 1u) {        var border = false;
        var offs = array<vec2<i32>, 8>(
            vec2<i32>(1,0),
            vec2<i32>(-1,0),
            vec2<i32>(0,1),
            vec2<i32>(0,-1),
            vec2<i32>(1,1),
            vec2<i32>(-1,1),
            vec2<i32>(1,-1),
            vec2<i32>(-1,-1),
        );
        for (var step: i32 = 1; step <= 4 && !border; step = step + 1) {
            for (var i: i32 = 0; i < 8; i = i + 1) {
                let uv = vec2<i32>(id.xy) + step * offs[i];
                if (uv.x < 0 || uv.y < 0 || uv.x >= i32(dims.x) || uv.y >= i32(dims.y)) { continue; }
                let n = i32(round(textureLoad(color_tex, uv, 0).w));
                if (n != obj_idx) { border = true; break; }
            }
        }
        if (border) {
            color = vec3<f32>(1.0, 1.0, 0.0);
        }
    }
    textureStore(out_tex, vec2<i32>(id.xy), vec4<f32>(color, 1.0));
}
