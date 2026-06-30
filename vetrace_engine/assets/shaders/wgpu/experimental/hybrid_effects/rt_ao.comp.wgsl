// Production decomposed hybrid ray-traced ambient occlusion pass.
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
    dof_aperture: f32, dof_focus_dist: f32, dof_enable: u32, _pad_dof: u32,
    atmosphere: u32, atmo_count: u32, cloud_count: u32, atmosphere_mode: u32,
    atmosphere_sun_controls: vec4<f32>,
    cloud_history_weight: f32, cloud_sample_count: u32, cloud_temporal_quality: u32, cloud_shadow_mode: u32,
    renderer_mode: u32, rt_debug_view: u32, rt_debug_counters: u32, max_traversal_steps: u32,
    max_transparent_surfaces: u32, shadow_max_distance: f32, reflection_max_distance: f32, gi_max_distance: f32,
    min_ray_offset: f32,
};

struct RtEffectParams {
    inv_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    enabled: u32,
    mode: u32,
    gi_mode: u32,
    rtao_sample_count: u32,
    rtao_radius_bits: u32,
    _pad_rt: u32,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<u32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var ao_out: texture_storage_2d<r16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;
@group(0) @binding(9) var<storage, read> objects: array<Object>;
@group(0) @binding(10) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(11) var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(12) var<storage, read> tri_bvh_nodes: array<TriBvhNode>;
@group(0) @binding(13) var<storage, read> materials: array<MaterialParams>;

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var world = rt_params.inv_view_proj * vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    return (world / max(world.w, 1.0e-6)).xyz;
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn hash11(n: u32) -> f32 {
    var x = n;
    x = (x ^ 61u) ^ (x >> 16u);
    x = x * 9u;
    x = x ^ (x >> 4u);
    x = x * 0x27d4eb2du;
    x = x ^ (x >> 15u);
    return f32(x & 0x00ffffffu) / 16777215.0;
}

fn tangent_basis(n: vec3<f32>) -> mat3x3<f32> {
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.95);
    let t = normalize(cross(up, n));
    let b = cross(n, t);
    return mat3x3<f32>(t, b, n);
}

fn cosine_hemisphere(u1: f32, u2: f32, n: vec3<f32>) -> vec3<f32> {
    let r = sqrt(u1);
    let phi = 6.28318530718 * u2;
    let local = vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - u1)));
    return normalize(tangent_basis(n) * local);
}

fn intersect_sphere(ro: vec3<f32>, rd: vec3<f32>, o: Object, tmax: f32) -> bool {
    let oc = ro - o.position;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - o.radius * o.radius;
    let h = b * b - c;
    if (h < 0.0) { return false; }
    let t = -b - sqrt(h);
    return t > T_EPS && t < tmax;
}

fn intersect_aabb(ro: vec3<f32>, rd: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>, tmax_limit: f32) -> bool {
    let inv_rd = 1.0 / max(abs(rd), vec3<f32>(1.0e-6)) * sign(rd);
    let t0 = (bmin - ro) * inv_rd;
    let t1 = (bmax - ro) * inv_rd;
    let tmin3 = min(t0, t1);
    let tmax3 = max(t0, t1);
    let tmin = max(max(tmin3.x, tmin3.y), tmin3.z);
    let tmax = min(min(tmax3.x, tmax3.y), tmax3.z);
    return tmax >= max(tmin, T_EPS) && tmin < tmax_limit;
}

fn intersect_triangle(ro: vec3<f32>, rd: vec3<f32>, tri: Triangle, tmax: f32) -> bool {
    let p = cross(rd, tri.e2);
    let det = dot(tri.e1, p);
    if (abs(det) < 1.0e-7) { return false; }
    let inv_det = 1.0 / det;
    let tvec = ro - tri.v0;
    let u = dot(tvec, p) * inv_det;
    if (u < 0.0 || u > 1.0) { return false; }
    let q = cross(tvec, tri.e1);
    let v = dot(rd, q) * inv_det;
    if (v < 0.0 || u + v > 1.0) { return false; }
    let t = dot(tri.e2, q) * inv_det;
    return t > T_EPS && t < tmax;
}


fn in_bounds_tlas(i: u32) -> bool { return i < params.total_bvh_nodes; }
fn in_bounds_tri_node(i: u32) -> bool { return i < params.total_tri_bvh_nodes; }
fn in_bounds_tri(i: u32) -> bool { return i < params.total_triangles; }

fn trace_mesh_occluder(ro: vec3<f32>, rd: vec3<f32>, obj: Object, tmax: f32) -> bool {
    if (obj.tri_bvh_count == 0u || obj.tri_bvh_start >= params.total_tri_bvh_nodes) { return false; }
    var steps = 0u;
    var stack: array<i32, 128>;
    var sp: i32 = 0;
    stack[sp] = i32(obj.tri_bvh_start); sp = sp + 1;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u;
        if (steps > max(params.max_traversal_steps, 1u)) { break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tri_node(ni)) { continue; }
        let node = tri_bvh_nodes[ni];
        if (!intersect_aabb(ro, rd, node.bmin.xyz, node.bmax.xyz, tmax)) { continue; }
        let c0 = node.child_tri.x;
        let c1 = node.child_tri.y;
        if (c0 < 0 && c1 < 0) {
            let ti = u32(node.child_tri.z);
            if (in_bounds_tri(ti) && intersect_triangle(ro, rd, triangles[ti], tmax)) { return true; }
        } else {
            if (c0 >= 0 && sp < 128) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 128) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return false;
}

fn trace_occluder(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> bool {
    if (params.total_bvh_nodes == 0u) { return false; }
    var steps = 0u;
    var stack: array<i32, 128>;
    var sp: i32 = 0;
    stack[sp] = 0; sp = sp + 1;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u;
        if (steps > max(params.max_traversal_steps, 1u)) { break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tlas(ni)) { continue; }
        let node = bvh_nodes[ni];
        if (!intersect_aabb(ro, rd, node.bmin.xyz, node.bmax.xyz, tmax)) { continue; }
        let c0 = node.child_object.x;
        let c1 = node.child_object.y;
        if (c0 < 0 && c1 < 0) {
            for (var lane = 0u; lane < 2u; lane = lane + 1u) {
                let oi = select(node.child_object.z, node.child_object.w, lane == 1u);
                if (oi < 0 || u32(oi) >= u32(max(params.num_objects, 0))) { continue; }
                let o = objects[u32(oi)];
                if (o.is_shaded == 0u) { continue; }
                if (o.is_mesh != 0u) {
                    if (trace_mesh_occluder(ro, rd, o, tmax)) { return true; }
                } else if (o.is_cube != 0u) {
                    let half_extent = max(o.size * o.scale * 0.5, vec3<f32>(0.0001));
                    if (intersect_aabb(ro, rd, o.position - half_extent, o.position + half_extent, tmax)) { return true; }
                } else if (intersect_sphere(ro, rd, o, tmax)) { return true; }
            }
        } else {
            if (c0 >= 0 && sp < 128) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 128) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return false;
}

fn ao_estimate(px: vec2<i32>, dims: vec2<u32>, sample_count: u32, radius: f32, seed_base: u32) -> f32 {
    let depth = textureLoad(depth_tex, px, 0).x;
    if (depth >= 0.9999) { return 1.0; }
    let n = unpack_normal(px);
    let p = reconstruct_world(px, dims, depth) + n * max(params.min_ray_offset, T_EPS);
    var visible = 0.0;
    for (var s: u32 = 0u; s < sample_count; s = s + 1u) {
        let u1 = hash11(seed_base + s * 2u + 17u);
        let u2 = hash11(seed_base + s * 2u + 53u);
        visible = visible + select(1.0, 0.0, trace_occluder(p, cosine_hemisphere(u1, u2, n), radius));
    }
    return clamp(visible / f32(max(sample_count, 1u)), 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let px = vec2<i32>(id.xy);
    let depth = textureLoad(depth_tex, px, 0).x;
    if (rt_params.enabled == 0u || depth >= 0.9999) {
        textureStore(ao_out, px, vec4<f32>(1.0, 0.0, 0.0, 1.0));
        return;
    }
    let radius = max(0.05, min(bitcast<f32>(rt_params.rtao_radius_bits), params.gi_max_distance));
    let samples = clamp(rt_params.rtao_sample_count, 1u, 16u);
    let seed = id.x * 1973u + id.y * 9277u + u32(max(params.frame_number, 0)) * 26699u;
    var ao = ao_estimate(px, dims, samples, radius, seed);
    let n0 = unpack_normal(px);
    let d0 = depth;
    var weight_sum = 1.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        var tap = vec2<i32>(0, 1);
        if (i == 0u) { tap = vec2<i32>(1, 0); }
        if (i == 1u) { tap = vec2<i32>(-1, 0); }
        if (i == 2u) { tap = vec2<i32>(0, 1); }
        if (i == 3u) { tap = vec2<i32>(0, -1); }
        let q = clamp(px + tap, vec2<i32>(0), vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1));
        let dq = textureLoad(depth_tex, q, 0).x;
        let nq = unpack_normal(q);
        let w = exp(-abs(dq - d0) * 64.0) * max(dot(n0, nq), 0.0);
        if (w > 0.15) {
            let qs = max(samples / 2u, 1u);
            ao = ao + ao_estimate(q, dims, qs, radius, seed + i * 101u + 409u) * w;
            weight_sum = weight_sum + w;
        }
    }
    textureStore(ao_out, px, vec4<f32>(clamp(ao / weight_sum, 0.0, 1.0), 0.0, 0.0, 1.0));
}
