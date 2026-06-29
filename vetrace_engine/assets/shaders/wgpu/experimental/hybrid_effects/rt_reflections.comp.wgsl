// Production decomposed hybrid RT reflections pass.
const T_EPS: f32 = 0.001;
const INF_T: f32 = 1.0e20;
const ROUGH_PROBE_ONLY: f32 = 0.60;
const ROUGH_RT_ALLOWED: f32 = 0.25;
const SSR_STEPS: i32 = 20;
const SSR_STRIDE: f32 = 10.0;
const SSR_THICKNESS: f32 = 0.35;

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

struct RtEffectParams { inv_view_proj: mat4x4<f32>, camera_pos: vec4<f32>, dir_light_dir: vec4<f32>, dir_light_color: vec4<f32>, enabled: u32, mode: u32, _pad: vec2<u32> };

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

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var world = rt_params.inv_view_proj * vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
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

struct MaterialData {
    base_color: vec3<f32>,
    alpha: f32,
    normal: vec3<f32>,
    roughness: f32,
    metallic: f32,
    transmission: f32,
    ior: f32,
    custom_flags: u32,
};

fn load_material_data(pixel: vec2<i32>) -> MaterialData {
    let albedo = textureLoad(albedo_tex, pixel, 0);
    let n = textureLoad(normal_tex, pixel, 0);
    let m = textureLoad(material_tex, pixel, 0);
    return MaterialData(
        albedo.rgb,
        albedo.a,
        normalize(n.xyz * 2.0 - vec3<f32>(1.0)),
        clamp(f32(m.g) / 255.0, 0.04, 1.0),
        f32(m.r) / 255.0,
        f32(m.b) / 255.0,
        max(n.w * 4.0, 1.0),
        m.a
    );
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn project_to_uv(world: vec3<f32>) -> vec2<f32> {
    // This experimental shader is not wired to receive a view-projection matrix yet.
    // Keep the shader valid by using a conservative centered projection placeholder
    // instead of the unsupported inverse() intrinsic on inv_view_proj.
    let view_dir = normalize(world - rt_params.camera_pos.xyz);
    return clamp(view_dir.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5), vec2<f32>(0.0), vec2<f32>(1.0));
}

fn probe_reflection(albedo: vec3<f32>, n: vec3<f32>, v: vec3<f32>, roughness: f32) -> vec3<f32> {
    // Cheap reflection-probe/cubemap stand-in: blend sky/directional ambient by the reflected lobe.
    // This is intentionally used before any RT work for rough walls/floors and SSR misses.
    let r = reflect(-v, n);
    let horizon = clamp(r.y * 0.5 + 0.5, 0.0, 1.0);
    let sky_probe = mix(rt_params.dir_light_color.rgb * 0.18, rt_params.dir_light_color.rgb, horizon);
    return mix(sky_probe, albedo, roughness * 0.65);
}

fn screen_space_reflection(pixel: vec2<i32>, dims: vec2<u32>, world: vec3<f32>, n: vec3<f32>, v: vec3<f32>, roughness: f32) -> vec4<f32> {
    let ray_dir = normalize(reflect(-v, n));
    var hit_confidence = 0.0;
    var hit_color = vec3<f32>(0.0);

    for (var i: i32 = 1; i <= SSR_STEPS; i = i + 1) {
        let t = f32(i) * SSR_STRIDE * (0.025 + roughness * 0.04);
        let sample_world = world + ray_dir * t;
        let uv = project_to_uv(sample_world);
        if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) { break; }
        let sp = vec2<i32>(uv * vec2<f32>(dims));
        let sd = textureLoad(depth_tex, sp, 0).x;
        if (sd >= 0.9999) { continue; }
        let scene_world = reconstruct_world(sp, dims, sd);
        let depth_error = abs(length(scene_world - rt_params.camera_pos.xyz) - length(sample_world - rt_params.camera_pos.xyz));
        let sn = unpack_normal(sp);
        let normal_ok = max(dot(sn, n), 0.0);
        if (depth_error < SSR_THICKNESS + roughness * 0.08 && normal_ok > 0.35) {
            hit_color = textureLoad(albedo_tex, sp, 0).rgb;
            hit_confidence = normal_ok * (1.0 - f32(i) / f32(SSR_STEPS + 1));
            break;
        }
    }
    return vec4<f32>(hit_color, hit_confidence);
}

fn rt_resolution_lane(pixel: vec2<i32>, roughness: f32) -> bool {
    if (rt_params.mode >= 2u) { return true; } // mirror/full-res mode
    if (rt_params.mode == 0u) { return ((pixel.x & 3) == 0) && ((pixel.y & 3) == 0); } // performance/quarter-res
    if (roughness >= ROUGH_RT_ALLOWED) { return ((pixel.x & 1) == 0) && ((pixel.y & 1) == 0); } // half-res mid roughness
    return ((pixel.x ^ pixel.y) & 1) == 0; // default half-rate for glossy surfaces
}

fn trace_reflection(ro: vec3<f32>, rd: vec3<f32>, max_t: f32, fallback: vec3<f32>) -> vec4<f32> {
    var best_t = max_t;
    var best_color = fallback;
    var hit = false;
    let count = min(u32(max(params.num_objects, 0)), 4096u);
    for (var i = 0u; i < count; i = i + 1u) {
        let obj = objects[i];
        if (obj.is_shaded == 0u) { continue; }
        var hit_t = INF_T;
        var mat_idx = obj.material_index;
        if (obj.is_mesh != 0u && obj.triangle_count > 0u) {
            let tri_end = min(obj.triangle_start_idx + obj.triangle_count, params.total_triangles);
            for (var ti = obj.triangle_start_idx; ti < tri_end; ti = ti + 1u) {
                let t = intersect_triangle(ro, rd, triangles[ti]);
                if (t < hit_t) { hit_t = t; mat_idx = triangles[ti].material_index; }
            }
        } else if (obj.is_cube != 0u) {
            let half_extent = max(obj.size * obj.scale * 0.5, vec3<f32>(0.0001));
            hit_t = intersect_aabb(ro, rd, obj.position - half_extent, obj.position + half_extent);
        } else {
            hit_t = intersect_sphere(ro, rd, obj);
        }
        if (hit_t < best_t) {
            best_t = hit_t;
            hit = true;
            best_color = materials[mat_idx].baseColorFactor.rgb + materials[mat_idx].emissiveFactor * materials[mat_idx].emissiveStrength;
        }
    }
    let fog = clamp(best_t / max(max_t, 0.001), 0.0, 1.0);
    return vec4<f32>(mix(best_color, fallback, fog * 0.35), select(0.0, 1.0 - fog * 0.5, hit));
}
fn miss(pixel: vec2<i32>) {
    textureStore(effect_out, pixel, vec4<f32>(0.0, 0.0, 0.0, 0.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (rt_params.enabled == 0u) { miss(pixel); return; }
    let depth = textureLoad(depth_tex, pixel, 0).x;
    if (depth >= 0.9999) { miss(pixel); return; }

    let world = reconstruct_world(pixel, dims, depth);
    let material = load_material_data(pixel);
    let n = material.normal;
    let albedo = material.base_color;
    let roughness = material.roughness;
    let object_id = textureLoad(object_id_tex, pixel, 0).r;
    let v = normalize(rt_params.camera_pos.xyz - world);
    let f0 = mix(vec3<f32>(0.04), albedo, material.metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - max(dot(n, v), 0.0), 5.0);

    let probe = probe_reflection(albedo, n, v, roughness);
    if (roughness > ROUGH_PROBE_ONLY) {
        textureStore(effect_out, pixel, vec4<f32>(probe * fresnel * 0.35, 0.0));
        return;
    }

    let ssr = screen_space_reflection(pixel, dims, world, n, v, roughness);
    let ssr_color = ssr.rgb * fresnel;
    if (ssr.a > 0.55) {
        textureStore(effect_out, pixel, vec4<f32>(mix(probe * fresnel, ssr_color, ssr.a), ssr.a));
        return;
    }

    let important_object = object_id != 0u;
    let insufficient_fallback = ssr.a < 0.25;
    let smooth_enough = roughness < ROUGH_RT_ALLOWED;
    let mid_roughness_blend = roughness >= ROUGH_RT_ALLOWED && roughness <= ROUGH_PROBE_ONLY;
    let rt_allowed = params.raytraced_reflections_enabled != 0u && params.reflection_max_distance > T_EPS && smooth_enough && important_object && insufficient_fallback && rt_resolution_lane(pixel, roughness);

    var reflection = mix(probe * fresnel, ssr_color, ssr.a);
    var confidence = max(ssr.a, 0.25);
    if (rt_allowed) {
        let rt_hit = trace_reflection(world + n * max(params.min_ray_offset, T_EPS), normalize(reflect(-v, n)), params.reflection_max_distance, probe);
        reflection = mix(probe, rt_hit.rgb, rt_hit.a) * fresnel * (1.0 - roughness * 0.5);
        confidence = max(rt_hit.a, 0.30);
    } else if (mid_roughness_blend) {
        reflection = mix(probe * fresnel, ssr_color, clamp(ssr.a * 0.75, 0.0, 0.5));
        confidence = max(confidence, 0.35);
    }

    textureStore(effect_out, pixel, vec4<f32>(reflection, confidence));
}
