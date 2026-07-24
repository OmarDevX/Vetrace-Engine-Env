enable wgpu_ray_query;

const T_EPS: f32 = 0.001;
const TERMINATE_ON_FIRST_HIT: u32 = 0x4u;
const SKIP_AABBS: u32 = 0x200u;
const RAY_QUERY_INTERSECTION_NONE: u32 = 0u;
const SHADOW_CASTER_MASK: u32 = 0x1u;

struct Params {
    camera_pos: vec4<f32>, camera_front: vec4<f32>, camera_up: vec4<f32>, camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>, fov: f32, num_objects: i32, is_fisheye: i32, _pad0: i32,
    skycolor: vec4<f32>, taa_jitter: vec2<f32>, current_time: f32, frame_number: i32,
    selected_index: i32, max_bounces: i32, light_samples: i32, dir_shadow_samples: i32,
    shadow_mode: u32, raytraced_shadows_enabled: u32, shadow_quality: u32, max_shadow_rays: u32,
    emissive_shadow_samples: u32, directional_shadow_samples: u32, cloud_object_shadows_enabled: u32,
    max_rt_shadow_distance: f32, rt_shadow_ray_t_max: f32, min_soft_shadow_radius: f32,
    raytraced_reflections_enabled: u32, _pad_reflections: u32,
    inv_view_proj: mat4x4<f32>, prev_view_proj: mat4x4<f32>,
    dir_light_dir: vec4<f32>, dir_light_color: vec4<f32>, sky_occlusion: f32,
    total_triangles: u32, total_bvh_nodes: u32, total_tri_bvh_nodes: u32,
};

struct RtEffectParams {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    enabled: u32,
    mode: u32,
    _pad: vec2<u32>,
};

struct RayInstanceMetadata {
    object_id: u32,
    material_table_offset: u32,
    submesh_table_offset: u32,
    flags: u32,
};

// Same screen-space inputs/output layout as rt_shadows.comp.wgsl so the
// renderer can swap this shader behind the existing RT-shadow pass.
@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<u32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var shadow_factor_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;

@group(1) @binding(0) var scene_tlas: acceleration_structure;
@group(1) @binding(1) var<storage, read> instance_metadata: array<RayInstanceMetadata>;

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip_xy = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var world = rt_params.inv_view_proj * vec4<f32>(clip_xy, depth, 1.0);
    return (world / world.w).xyz;
}

fn trace_shadow_ray_query(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    let tmax = min(params.max_rt_shadow_distance, params.rt_shadow_ray_t_max);
    if (tmax <= T_EPS) {
        return 1.0;
    }

    var rq: ray_query;
    rayQueryInitialize(
        &rq,
        scene_tlas,
        RayDesc(
            TERMINATE_ON_FIRST_HIT | SKIP_AABBS,
            SHADOW_CASTER_MASK,
            T_EPS,
            tmax,
            ro,
            rd,
        ),
    );
    while rayQueryProceed(&rq) {}

    let hit = rayQueryGetCommittedIntersection(&rq);
    return select(0.0, 1.0, hit.kind == RAY_QUERY_INTERSECTION_NONE);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }

    let pixel = vec2<i32>(id.xy);
    if (rt_params.enabled == 0u || params.raytraced_shadows_enabled == 0u) {
        textureStore(shadow_factor_out, pixel, vec4<f32>(1.0));
        return;
    }

    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) {
        textureStore(shadow_factor_out, pixel, vec4<f32>(1.0));
        return;
    }

    let n = normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
    let l = normalize(-params.dir_light_dir.xyz);
    if (dot(n, l) <= 0.0) {
        textureStore(shadow_factor_out, pixel, vec4<f32>(1.0));
        return;
    }

    let world = reconstruct_world(pixel, dims, depth);
    let shadow = trace_shadow_ray_query(world + n * T_EPS, l);
    textureStore(shadow_factor_out, pixel, vec4<f32>(shadow, shadow, shadow, 1.0));
}
