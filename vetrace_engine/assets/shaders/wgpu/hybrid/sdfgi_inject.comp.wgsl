// Inject directional light into radiance volume using SDF for occlusion
struct Params {
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_up: vec4<f32>,
    camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>,
    fov: f32,
    num_objects: i32,
    is_fisheye: i32,
    _pad0: i32,
    skycolor: vec4<f32>,
    taa_jitter: vec2<f32>,
    current_time: f32,
    frame_number: i32,
    selected_index: i32,
    max_bounces: i32,
    light_samples: i32,
    dir_shadow_samples: i32,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    sky_occlusion: f32,
    total_triangles: u32,
    total_bvh_nodes: u32,
    total_tri_bvh_nodes: u32,
};


@group(0) @binding(0) var sdf_tex: texture_3d<f32>;
@group(0) @binding(1) var sdf_sampler: sampler;
@group(0) @binding(2) var radiance_tex: texture_storage_3d<rgba16float, write>;
@group(0) @binding(3) var<uniform> params: Params;

const SDF_MIN: vec3<f32> = vec3<f32>(-10.0, -10.0, -10.0);
const SDF_SIZE: vec3<f32> = vec3<f32>(20.0, 20.0, 20.0);

fn world_to_sdf(p: vec3<f32>) -> vec3<f32> {
    return (p - SDF_MIN) / SDF_SIZE;
}

fn visible_to_light(p: vec3<f32>) -> bool {
    let dir = normalize(-params.dir_light_dir.xyz);
    let max_dist = length(SDF_SIZE);
    var t = 0.0;
    for (var i: u32 = 0u; i < 32u; i = i + 1u) {
        let pos = p + dir * t;
        let uv = world_to_sdf(pos);
        let d = textureSampleLevel(sdf_tex, sdf_sampler, uv, 0.0).x;
        if (d < 0.0) { return false; }
        t += max(d, 0.1);
        if (t > max_dist) { break; }
    }
    return true;
}

@compute @workgroup_size(8,8,4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(radiance_tex);
    if (id.x >= dims.x || id.y >= dims.y || id.z >= dims.z) { return; }
    let rel = vec3<f32>(id) / vec3<f32>(dims);
    let world_pos = SDF_MIN + rel * SDF_SIZE;
    var rad = vec3<f32>(0.0);
    if (params.dir_light_dir.w > 0.0 && visible_to_light(world_pos)) {
        rad = params.dir_light_color.xyz * params.dir_light_dir.w;
    }
    textureStore(radiance_tex, vec3<i32>(id), vec4<f32>(rad, 0.0));
}