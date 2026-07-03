// Dedicated screen-space RT shadow factor pass for RendererMode::HybridEffects.
// Reads production primitive_gbuffer/pbr_gbuffer outputs and traces scene data shared
// with hybrid/pathtrace.comp.wgsl.

const T_EPS: f32 = 0.001;
const INF_T: f32 = 1.0e20;

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
    _struct_pad0: u32,
    _struct_pad1: u32,
};

struct Triangle {
    v0: vec3<f32>, _pad0: f32,
    e1: vec3<f32>, _pad1: f32,
    e2: vec3<f32>, _pad2: f32,
    n0: vec3<f32>, _pad3: f32,
    n1: vec3<f32>, _pad4: f32,
    n2: vec3<f32>, _pad5: f32,
    uv0: vec2<f32>, duv1: vec2<f32>,
    duv2: vec2<f32>, material_index: u32, _pad6: u32,
};

struct BvhNode { bmin: vec4<f32>, bmax: vec4<f32>, child_object: vec4<i32> };
struct TriBvhNode { bmin: vec4<f32>, bmax: vec4<f32>, child_tri: vec4<i32> };
struct MaterialParams {
    baseColorFactor: vec4<f32>,
    emissiveFactor: vec3<f32>, emissiveStrength: f32,
    metallicFactor: f32, roughnessFactor: f32, ior: f32, baseColorTex: u32,
    f0: vec3<f32>, has_custom_material: u32,
    custom_material_id: u32,
    material_flags0: u32, material_flags1: u32, material_flags2: u32, material_flags3: u32,
    material_flags4: u32, material_flags5: u32, material_flags6: u32,
};

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

struct RtEffectParams { inv_view_proj: mat4x4<f32>, camera_pos: vec4<f32>, dir_light_dir: vec4<f32>, dir_light_color: vec4<f32>, enabled: u32, mode: u32, _pad: vec2<u32> };

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<u32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var shadow_factor_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;
@group(0) @binding(9) var<storage, read> objects: array<Object>;
@group(0) @binding(10) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(11) var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(12) var<storage, read> tri_bvh_nodes: array<TriBvhNode>;
@group(0) @binding(13) var<storage, read> materials: array<MaterialParams>;

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip_xy = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var world = rt_params.inv_view_proj * vec4<f32>(clip_xy, depth, 1.0);
    return (world / world.w).xyz;
}

fn intersect_sphere(ro: vec3<f32>, rd: vec3<f32>, o: Object) -> f32 {
    let oc = ro - o.position;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - o.radius * o.radius;
    let h = b * b - c;
    if (h < 0.0) { return INF_T; }
    let t = -b - sqrt(h);
    return select(INF_T, t, t > T_EPS);
}

fn intersect_aabb(ro: vec3<f32>, rd: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> f32 {
    let inv_rd = 1.0 / max(abs(rd), vec3<f32>(1.0e-6)) * sign(rd);
    let t0 = (bmin - ro) * inv_rd;
    let t1 = (bmax - ro) * inv_rd;
    let tmin3 = min(t0, t1);
    let tmax3 = max(t0, t1);
    let tmin = max(max(tmin3.x, tmin3.y), tmin3.z);
    let tmax = min(min(tmax3.x, tmax3.y), tmax3.z);
    return select(INF_T, max(tmin, T_EPS), tmax >= max(tmin, T_EPS));
}

fn intersect_triangle(ro: vec3<f32>, rd: vec3<f32>, tri: Triangle) -> f32 {
    let p = cross(rd, tri.e2);
    let det = dot(tri.e1, p);
    if (abs(det) < 1.0e-7) { return INF_T; }
    let inv_det = 1.0 / det;
    let tvec = ro - tri.v0;
    let u = dot(tvec, p) * inv_det;
    if (u < 0.0 || u > 1.0) { return INF_T; }
    let q = cross(tvec, tri.e1);
    let v = dot(rd, q) * inv_det;
    if (v < 0.0 || u + v > 1.0) { return INF_T; }
    let t = dot(tri.e2, q) * inv_det;
    return select(INF_T, t, t > T_EPS);
}

fn trace_shadow(ro: vec3<f32>, rd: vec3<f32>) -> f32 {
    var factor = 1.0;
    let count = min(u32(max(params.num_objects, 0)), 4096u);
    for (var i = 0u; i < count; i = i + 1u) {
        let obj = objects[i];
        if (obj.casts_raytraced_shadow == 0u || obj.shadow_importance <= 0.0) { continue; }
        let max_obj_t = min(max(obj.max_shadow_distance, 0.0), min(params.max_rt_shadow_distance, params.rt_shadow_ray_t_max));
        if (max_obj_t <= T_EPS) { continue; }
        var hit_t = INF_T;
        if (obj.is_mesh != 0u && obj.triangle_count > 0u) {
            let tri_end = min(obj.triangle_start_idx + obj.triangle_count, params.total_triangles);
            for (var ti = obj.triangle_start_idx; ti < tri_end; ti = ti + 1u) { hit_t = min(hit_t, intersect_triangle(ro, rd, triangles[ti])); }
        } else if (obj.is_cube != 0u) {
            let half_extent = max(obj.size * obj.scale * 0.5, vec3<f32>(0.0001));
            hit_t = intersect_aabb(ro, rd, obj.position - half_extent, obj.position + half_extent);
        } else {
            hit_t = intersect_sphere(ro, rd, obj);
        }
        if (hit_t < max_obj_t) { factor = min(factor, clamp(1.0 - obj.shadow_importance, 0.0, 1.0)); }
    }
    return factor;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (rt_params.enabled == 0u || params.raytraced_shadows_enabled == 0u) { textureStore(shadow_factor_out, pixel, vec4<f32>(1.0)); return; }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) { textureStore(shadow_factor_out, pixel, vec4<f32>(1.0)); return; }
    let n = normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
    let l = normalize(-params.dir_light_dir.xyz);
    let ndotl = dot(n, l);
    if (ndotl <= 0.0) { textureStore(shadow_factor_out, pixel, vec4<f32>(1.0)); return; }
    let world = reconstruct_world(pixel, dims, depth);
    let shadow = trace_shadow(world + n * T_EPS, l);
    textureStore(shadow_factor_out, pixel, vec4<f32>(shadow, shadow, shadow, 1.0));
}
