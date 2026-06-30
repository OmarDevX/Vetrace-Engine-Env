// Shared BVH storage declarations and traversal helpers for decomposed hybrid RT passes.
// WGSL has no native include; Rust concatenates this file before each pass-specific shader.
// BVH leaves are interpreted consistently across RT reflections, RTGI, and RTAO:
// TLAS leaf: child_object.x/y < 0 and child_object.z/w contain up to two object indices.
// BLAS leaf: child_tri.x/y < 0 and child_tri.z contains one triangle index.

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

struct Hit { t: f32, material_index: u32, normal: vec3<f32>, hit: u32, pos: vec3<f32> };

@group(0) @binding(9) var<storage, read> objects: array<Object>;
@group(0) @binding(10) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(11) var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(12) var<storage, read> tri_bvh_nodes: array<TriBvhNode>;
@group(0) @binding(13) var<storage, read> materials: array<MaterialParams>;

fn intersect_sphere(ro: vec3<f32>, rd: vec3<f32>, o: Object) -> f32 {
    let oc = ro - o.position; let b = dot(oc, rd); let c = dot(oc, oc) - o.radius * o.radius; let h = b * b - c;
    if (h < 0.0) { return INF_T; }
    let t = -b - sqrt(h); return select(INF_T, t, t > T_EPS);
}
fn intersect_aabb(ro: vec3<f32>, rd: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> f32 {
    let inv_rd = 1.0 / max(abs(rd), vec3<f32>(1.0e-6)) * sign(rd);
    let t0 = (bmin - ro) * inv_rd; let t1 = (bmax - ro) * inv_rd; let mn = min(t0, t1); let mx = max(t0, t1);
    let tmin = max(max(mn.x, mn.y), mn.z); let tmax = min(min(mx.x, mx.y), mx.z);
    return select(INF_T, max(tmin, T_EPS), tmax >= max(tmin, T_EPS));
}
fn intersect_triangle(ro: vec3<f32>, rd: vec3<f32>, tri: Triangle) -> vec4<f32> {
    let p = cross(rd, tri.e2); let det = dot(tri.e1, p);
    if (abs(det) < 1.0e-7) { return vec4<f32>(INF_T, 0.0, 0.0, 0.0); }
    let inv_det = 1.0 / det; let tvec = ro - tri.v0; let u = dot(tvec, p) * inv_det;
    if (u < 0.0 || u > 1.0) { return vec4<f32>(INF_T, 0.0, 0.0, 0.0); }
    let q = cross(tvec, tri.e1); let v = dot(rd, q) * inv_det;
    if (v < 0.0 || u + v > 1.0) { return vec4<f32>(INF_T, 0.0, 0.0, 0.0); }
    let t = dot(tri.e2, q) * inv_det; return vec4<f32>(select(INF_T, t, t > T_EPS), u, v, 0.0);
}
fn in_bounds_tlas(i: u32) -> bool { return i < params.total_bvh_nodes; }
fn in_bounds_tri_node(i: u32) -> bool { return i < params.total_tri_bvh_nodes; }
fn in_bounds_tri(i: u32) -> bool { return i < params.total_triangles; }

fn trace_mesh(ro: vec3<f32>, rd: vec3<f32>, obj: Object, best_t: f32) -> Hit {
    var best = Hit(best_t, obj.material_index, vec3<f32>(0.0, 1.0, 0.0), 0u, vec3<f32>(0.0));
    if (obj.tri_bvh_count == 0u || obj.tri_bvh_start >= params.total_tri_bvh_nodes) { return best; }
    var steps = 0u; var stack: array<i32, 128>; var sp: i32 = 0; stack[sp] = i32(obj.tri_bvh_start); sp = sp + 1;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u; if (steps > max(params.max_traversal_steps, 1u)) { break; }
        sp = sp - 1; let ni = u32(stack[sp]); if (!in_bounds_tri_node(ni)) { continue; }
        let node = tri_bvh_nodes[ni]; if (intersect_aabb(ro, rd, node.bmin.xyz, node.bmax.xyz) >= best.t) { continue; }
        let c0 = node.child_tri.x; let c1 = node.child_tri.y;
        if (c0 < 0 && c1 < 0) {
            let ti = u32(node.child_tri.z);
            if (in_bounds_tri(ti)) {
                let tri = triangles[ti]; let res = intersect_triangle(ro, rd, tri);
                if (res.x < best.t) {
                    let w = 1.0 - res.y - res.z;
                    best = Hit(res.x, select(obj.material_index, tri.material_index, tri.material_index != 0u), normalize(tri.n0 * w + tri.n1 * res.y + tri.n2 * res.z), 1u, ro + rd * res.x);
                }
            }
        } else {
            if (c0 >= 0 && sp < 128) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 128) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return best;
}

fn trace_scene_limit(ro: vec3<f32>, rd: vec3<f32>, max_t: f32) -> Hit {
    var best = Hit(max_t, 0u, vec3<f32>(0.0, 1.0, 0.0), 0u, vec3<f32>(0.0));
    if (params.total_bvh_nodes == 0u) { return best; }
    var steps = 0u; var stack: array<i32, 128>; var sp: i32 = 0; stack[sp] = 0; sp = sp + 1;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u; if (steps > max(params.max_traversal_steps, 1u)) { break; }
        sp = sp - 1; let ni = u32(stack[sp]); if (!in_bounds_tlas(ni)) { continue; }
        let node = bvh_nodes[ni]; if (intersect_aabb(ro, rd, node.bmin.xyz, node.bmax.xyz) >= best.t) { continue; }
        let c0 = node.child_object.x; let c1 = node.child_object.y;
        if (c0 < 0 && c1 < 0) {
            for (var lane = 0u; lane < 2u; lane = lane + 1u) {
                let oi = select(node.child_object.z, node.child_object.w, lane == 1u);
                if (oi < 0 || u32(oi) >= u32(max(params.num_objects, 0))) { continue; }
                let obj = objects[u32(oi)]; if (obj.is_shaded == 0u) { continue; }
                if (obj.is_mesh != 0u) { let mh = trace_mesh(ro, rd, obj, best.t); if (mh.hit != 0u && mh.t < best.t) { best = mh; } }
                else {
                    let half_extent = max(obj.size * obj.scale * 0.5, vec3<f32>(0.0001));
                    let t = select(intersect_sphere(ro, rd, obj), intersect_aabb(ro, rd, obj.position - half_extent, obj.position + half_extent), obj.is_cube != 0u);
                    if (t < best.t) { let hp = ro + rd * t; let gn = select(normalize(hp - obj.position), normalize((hp - obj.position) / half_extent), obj.is_cube != 0u); best = Hit(t, obj.material_index, gn, 1u, hp); }
                }
            }
        } else {
            if (c0 >= 0 && sp < 128) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 128) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return best;
}
fn trace_scene(ro: vec3<f32>, rd: vec3<f32>, _max_objects: u32, _max_tris_per_mesh: u32) -> Hit { return trace_scene_limit(ro, rd, min(params.gi_max_distance, INF_T)); }
fn trace_occluder(ro: vec3<f32>, rd: vec3<f32>, tmax: f32) -> bool { return trace_scene_limit(ro, rd, tmax).hit != 0u; }
