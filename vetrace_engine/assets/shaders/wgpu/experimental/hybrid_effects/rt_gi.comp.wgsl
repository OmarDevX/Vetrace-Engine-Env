// Production-active decomposed hybrid one-bounce RTGI effect pass.
const GI_MODE_RTGI_ONE_BOUNCE: u32 = 4u;
const T_EPS: f32 = 0.002;
const INF_T: f32 = 1.0e20;
const PI: f32 = 3.14159265359;

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

struct Hit { t: f32, material_index: u32, normal: vec3<f32>, hit: u32, pos: vec3<f32> };

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<u32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var effect_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;
@group(0) @binding(9) var<storage, read> objects: array<Object>;
@group(0) @binding(10) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(11) var<storage, read> bvh_nodes: array<BvhNode>;
@group(0) @binding(12) var<storage, read> tri_bvh_nodes: array<TriBvhNode>;
@group(0) @binding(13) var<storage, read> materials: array<MaterialParams>;

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> { return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0)); }
fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var world = rt_params.inv_view_proj * vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    return (world / world.w).xyz;
}
fn hash12(p: vec2<u32>, salt: u32) -> f32 {
    var x = p.x * 1664525u + p.y * 1013904223u + u32(params.frame_number) * 747796405u + salt;
    x = ((x >> 16u) ^ x) * 2246822519u;
    x = ((x >> 13u) ^ x) * 3266489917u;
    return f32((x >> 8u) & 16777215u) / 16777215.0;
}
fn cosine_dir(n: vec3<f32>, pixel: vec2<u32>, sample_idx: u32) -> vec3<f32> {
    let r1 = hash12(pixel, 0x9e3779b9u + sample_idx * 17u);
    let r2 = hash12(pixel, 0x85ebca6bu + sample_idx * 31u);
    let phi = 2.0 * PI * r1;
    let r = sqrt(r2);
    let local = vec3<f32>(cos(phi) * r, sin(phi) * r, sqrt(max(0.0, 1.0 - r2)));
    let up = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.95);
    let t = normalize(cross(up, n));
    let b = cross(n, t);
    return normalize(t * local.x + b * local.y + n * local.z);
}
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
    var steps = 0u;
    var stack: array<i32, 128>; var sp: i32 = 0;
    stack[sp] = i32(obj.tri_bvh_start); sp = sp + 1;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u;
        if (steps > max(params.max_traversal_steps, 1u)) { break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tri_node(ni)) { continue; }
        let node = tri_bvh_nodes[ni];
        if (intersect_aabb(ro, rd, node.bmin.xyz, node.bmax.xyz) >= best.t) { continue; }
        let c0 = node.child_tri.x; let c1 = node.child_tri.y;
        if (c0 < 0 && c1 < 0) {
            let ti = u32(node.child_tri.z);
            if (in_bounds_tri(ti)) {
                let tri = triangles[ti];
                let res = intersect_triangle(ro, rd, tri);
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

fn trace_scene(ro: vec3<f32>, rd: vec3<f32>, _max_objects: u32, _max_tris_per_mesh: u32) -> Hit {
    var best = Hit(min(params.gi_max_distance, INF_T), 0u, vec3<f32>(0.0, 1.0, 0.0), 0u, vec3<f32>(0.0));
    if (params.total_bvh_nodes == 0u) { return best; }
    var steps = 0u;
    var stack: array<i32, 128>; var sp: i32 = 0;
    stack[sp] = 0; sp = sp + 1;
    loop {
        if (sp == 0) { break; }
        steps = steps + 1u;
        if (steps > max(params.max_traversal_steps, 1u)) { break; }
        sp = sp - 1;
        let ni = u32(stack[sp]);
        if (!in_bounds_tlas(ni)) { continue; }
        let node = bvh_nodes[ni];
        if (intersect_aabb(ro, rd, node.bmin.xyz, node.bmax.xyz) >= best.t) { continue; }
        let c0 = node.child_object.x; let c1 = node.child_object.y;
        if (c0 < 0 && c1 < 0) {
            for (var lane = 0u; lane < 2u; lane = lane + 1u) {
                let oi = select(node.child_object.z, node.child_object.w, lane == 1u);
                if (oi < 0 || u32(oi) >= u32(max(params.num_objects, 0))) { continue; }
                let obj = objects[u32(oi)]; if (obj.is_shaded == 0u) { continue; }
                if (obj.is_mesh != 0u) {
                    let mh = trace_mesh(ro, rd, obj, best.t);
                    if (mh.hit != 0u && mh.t < best.t) { best = mh; }
                } else {
                    let half_extent = max(obj.size * obj.scale * 0.5, vec3<f32>(0.0001));
                    let t = select(intersect_sphere(ro, rd, obj), intersect_aabb(ro, rd, obj.position - half_extent, obj.position + half_extent), obj.is_cube != 0u);
                    if (t < best.t) {
                        let hp = ro + rd * t;
                        let gn = select(normalize(hp - obj.position), normalize((hp - obj.position) / half_extent), obj.is_cube != 0u);
                        best = Hit(t, obj.material_index, gn, 1u, hp);
                    }
                }
            }
        } else {
            if (c0 >= 0 && sp < 128) { stack[sp] = c0; sp = sp + 1; }
            if (c1 >= 0 && sp < 128) { stack[sp] = c1; sp = sp + 1; }
        }
    }
    return best;
}
fn visible_to_light(pos: vec3<f32>, n: vec3<f32>, l: vec3<f32>, max_objects: u32) -> f32 {
    if (dot(n, l) <= 0.0) { return 0.0; }
    let h = trace_scene(pos + n * T_EPS, l, max_objects, 64u);
    return select(1.0, 0.0, h.hit != 0u && h.t < min(params.max_rt_shadow_distance, params.rt_shadow_ray_t_max));
}
fn sky_radiance(rd: vec3<f32>) -> vec3<f32> {
    let horizon = clamp(rd.y * 0.5 + 0.5, 0.0, 1.0);
    return params.skycolor.rgb * (0.35 + 0.65 * horizon) * max(0.0, 1.0 - params.sky_occlusion);
}
fn material_radiance(hit: Hit, hit_pos: vec3<f32>, max_objects: u32) -> vec3<f32> {
    let mat = materials[hit.material_index];
    var albedo = mat.baseColorFactor.rgb;
    let clip = rt_params.view_proj * vec4<f32>(hit_pos, 1.0);
    if (mat.baseColorTex != 0u && clip.w > 0.0) {
        let ndc = clip.xyz / clip.w;
        let uv = ndc.xy * 0.5 + vec2<f32>(0.5);
        let dims = textureDimensions(albedo_tex);
        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
            let spx = vec2<i32>(clamp(uv * vec2<f32>(dims), vec2<f32>(0.0), vec2<f32>(dims - vec2<u32>(1u))));
            let sd = textureLoad(depth_tex, spx, 0).x;
            if (abs(sd - ndc.z) < 0.01) { albedo = textureLoad(albedo_tex, spx, 0).rgb; }
        }
    }
    let emissive = mat.emissiveFactor * mat.emissiveStrength;
    let l = normalize(-params.dir_light_dir.xyz);
    let ndotl = max(dot(hit.normal, l), 0.0);
    let vis = visible_to_light(hit_pos + hit.normal * T_EPS, hit.normal, l, max_objects);
    let direct = params.dir_light_color.rgb * ndotl * vis;
    return emissive + albedo * direct;
}
fn write_miss(pixel: vec2<i32>) { textureStore(effect_out, pixel, vec4<f32>(0.0)); }

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (rt_params.enabled == 0u || rt_params.gi_mode != GI_MODE_RTGI_ONE_BOUNCE) { write_miss(pixel); return; }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) { write_miss(pixel); return; }
    let n = unpack_normal(pixel);
    let world = reconstruct_world(pixel, dims, depth);
    let surface_albedo = textureLoad(albedo_tex, pixel, 0).rgb;
    let adaptive_samples = u32(max(params.light_samples, 1));
    let high_quality = adaptive_samples >= 2u && params.max_bounces > 1;
    let rays = select(1u, 2u, high_quality);
    let max_objects = select(128u, 512u, high_quality);
    let max_tris = select(128u, 1024u, high_quality);
    var sum = vec3<f32>(0.0);
    for (var s = 0u; s < rays; s = s + 1u) {
        let rd = cosine_dir(n, id.xy, s);
        let hit = trace_scene(world + n * max(params.min_ray_offset, T_EPS), rd, max_objects, max_tris);
        var incoming = sky_radiance(rd);
        if (hit.hit != 0u) {
            incoming = material_radiance(hit, hit.pos, max_objects);
        }
        sum = sum + incoming;
    }
    let irradiance = surface_albedo * sum / f32(rays);
    textureStore(effect_out, pixel, vec4<f32>(max(irradiance, vec3<f32>(0.0)), 1.0));
}
