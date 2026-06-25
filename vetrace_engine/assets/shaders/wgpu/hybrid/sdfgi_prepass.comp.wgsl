// Prepass builds a signed distance field for global illumination
struct Object {
    orientation: vec4<f32>,
    position: vec3<f32>, _pad1: f32,
    size: vec3<f32>, _pad2: f32,
    scale: vec3<f32>, _pad2b: f32,
    material_index: u32,
    radius: f32,
    is_cube: u32,
    is_glass: u32,
    is_mesh: u32,
    triangle_start_idx: u32,
    triangle_count: u32,
    tri_bvh_start: u32,
    tri_bvh_count: u32,
    is_shaded: u32,
    casts_raster_shadow: u32,
    casts_raytraced_shadow: u32,
    shadow_importance: f32,
    max_shadow_distance: f32,
    scene_flags: u32,
    gi_flags: u32,
    _gi_pad0: u32,
    _gi_pad1: u32,
};

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
    max_bounces: i32,
    light_samples: i32,
    dir_shadow_samples: i32,
    shadow_mode: u32,
    raytraced_shadows_enabled: u32,
    shadow_quality: u32,
    max_shadow_rays: u32,
    emissive_shadow_samples: u32,
    directional_shadow_samples: u32,
    cloud_object_shadows_enabled: u32,
    max_rt_shadow_distance: f32,
    rt_shadow_ray_t_max: f32,
    min_soft_shadow_radius: f32,
    _pad_shadow_mode: u32,
    inv_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<storage, read> objects: array<Object>;
@group(0) @binding(1) var<uniform> params: Params;
@group(0) @binding(2) var sdf_tex: texture_storage_3d<r32float, write>;

const SDF_MIN: vec3<f32> = vec3<f32>(-10.0, -10.0, -10.0);
const SDF_SIZE: vec3<f32> = vec3<f32>(20.0, 20.0, 20.0);

fn quat_conjugate(q: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(-q.xyz, q.w);
}
fn quat_normalize(q: vec4<f32>) -> vec4<f32> {
    return q * inverseSqrt(dot(q, q));
}
fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let nq = quat_normalize(q);
    return v + 2.0 * cross(nq.xyz, cross(nq.xyz, v) + nq.w * v);
}

fn sdf_cube(p: vec3<f32>, pos: vec3<f32>, size: vec3<f32>, orient: vec4<f32>, scale: vec3<f32>) -> f32 {
    let inv = quat_conjugate(quat_normalize(orient));
    let lp = quat_rotate(inv, p - pos);
    let d = abs(lp) - (size * scale) * 0.5;
    let out_dist = length(max(d, vec3<f32>(0.0)));
    let in_dist = min(max(d.x, max(d.y, d.z)), 0.0);
    return out_dist + in_dist;
}
fn sdf_sphere(p: vec3<f32>, pos: vec3<f32>, r: f32) -> f32 {
    return length(p - pos) - r;
}

fn object_sdf(p: vec3<f32>, obj: Object) -> f32 {
    if (obj.is_cube > 0u) { return sdf_cube(p, obj.position, obj.size, obj.orientation, obj.scale); }
    return sdf_sphere(p, obj.position, obj.radius);
}

@compute @workgroup_size(8,8,4)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(sdf_tex);
    if (id.x >= dims.x || id.y >= dims.y || id.z >= dims.z) { return; }
    let rel = vec3<f32>(id) / vec3<f32>(dims);
    let world_pos = SDF_MIN + rel * SDF_SIZE;
    var dist = 1e9;
    for (var i: u32 = 0u; i < u32(params.num_objects); i = i + 1u) {
        let obj = objects[i];
        if ((obj.scene_flags & 2u) == 0u) {
            dist = min(dist, object_sdf(world_pos, obj));
        }
    }
    let maxR = length(SDF_SIZE);
    dist = clamp(dist, -maxR, maxR);
    textureStore(sdf_tex, vec3<i32>(id), vec4(dist, 0.0, 0.0, 0.0));
}