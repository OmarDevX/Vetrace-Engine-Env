// Production decomposed hybrid RT reflections pass.
const T_EPS: f32 = 0.001;
const INF_T: f32 = 1.0e20;
const ROUGH_PROBE_ONLY: f32 = 0.60;
const ROUGH_RT_ALLOWED: f32 = 0.25;
const SSR_HIGH_CONFIDENCE: f32 = 0.65;
const MATERIAL_FLAG_ACCURATE_REFLECTION: u32 = 0x1u;

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

struct RtEffectParams { inv_view_proj: mat4x4<f32>, view_proj: mat4x4<f32>, camera_pos: vec4<f32>, dir_light_dir: vec4<f32>, dir_light_color: vec4<f32>, enabled: u32, mode: u32, _pad: vec2<u32> };

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<u32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var effect_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;
@group(0) @binding(8) var<uniform> params: Params;
// Shared BVH declarations/traversal are concatenated by Rust from hybrid/bvh_traversal.wgsl.
@group(0) @binding(14) var ssr_tex: texture_2d<f32>;
fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip_xy = uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    var world = rt_params.inv_view_proj * vec4<f32>(clip_xy, depth, 1.0);
    return (world / world.w).xyz;
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


fn probe_reflection(albedo: vec3<f32>, n: vec3<f32>, v: vec3<f32>, roughness: f32) -> vec3<f32> {
    // Cheap reflection-probe/cubemap stand-in: blend sky/directional ambient by the reflected lobe.
    // This is intentionally used before any RT work for rough walls/floors and SSR misses.
    let r = reflect(-v, n);
    let horizon = clamp(r.y * 0.5 + 0.5, 0.0, 1.0);
    let sky_probe = mix(rt_params.dir_light_color.rgb * 0.18, rt_params.dir_light_color.rgb, horizon);
    return mix(sky_probe, albedo, roughness * 0.65);
}

fn rt_resolution_lane(pixel: vec2<i32>, roughness: f32, needs_accurate_reflection: bool, mirror_like: bool) -> bool {
    // Mirror/accurate materials must be full-rate. Checkerboard RT on a mirror
    // produces the exact "SSR + RT mixed"/missing-strip look the editor test shows.
    if (needs_accurate_reflection || mirror_like || rt_params.mode >= 2u) { return true; }
    if (rt_params.mode == 0u) { return ((pixel.x & 3) == 0) && ((pixel.y & 3) == 0); } // performance/quarter-res
    if (roughness >= ROUGH_RT_ALLOWED) { return ((pixel.x & 1) == 0) && ((pixel.y & 1) == 0); } // half-res mid roughness
    return ((pixel.x ^ pixel.y) & 1) == 0; // default half-rate for glossy surfaces
}

fn trace_reflection(ro: vec3<f32>, rd: vec3<f32>, max_t: f32, fallback: vec3<f32>) -> vec4<f32> {
    let hit = trace_scene_limit(ro, rd, max_t);
    if (hit.hit == 0u) { return vec4<f32>(fallback, 0.0); }
    let mat = materials[hit.material_index];
    let hit_color = mat.baseColorFactor.rgb + mat.emissiveFactor * mat.emissiveStrength;
    let fog = clamp(hit.t / max(max_t, 0.001), 0.0, 1.0);
    return vec4<f32>(mix(hit_color, fallback, fog * 0.35), 1.0 - fog * 0.5);
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
    let ssr = textureLoad(ssr_tex, pixel, 0);
    let ssr_confidence = clamp(ssr.a, 0.0, 1.0);
    let ssr_color = ssr.rgb * fresnel;
    let needs_accurate_reflection = (material.custom_flags & MATERIAL_FLAG_ACCURATE_REFLECTION) != 0u || object_id != 0u;
    let mirror_like = roughness <= 0.08;
    let probe_fallback_acceptable = roughness >= ROUGH_PROBE_ONLY && !needs_accurate_reflection && !mirror_like;
    let high_confidence_ssr = ssr_confidence >= SSR_HIGH_CONFIDENCE && !needs_accurate_reflection && !mirror_like;

    if (probe_fallback_acceptable) {
        // Probe/final blend is owned by hybrid_compose; RT output stays RT-only.
        textureStore(effect_out, pixel, vec4<f32>(0.0));
        return;
    }
    if (high_confidence_ssr) {
        // SSR owns this pixel; keep the RT output/history separate by writing no RT contribution.
        textureStore(effect_out, pixel, vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    let low_confidence_ssr = ssr_confidence < SSR_HIGH_CONFIDENCE;
    let smooth_enough = roughness < ROUGH_RT_ALLOWED || mirror_like || needs_accurate_reflection;
    let rt_allowed = params.raytraced_reflections_enabled != 0u
        && params.reflection_max_distance > T_EPS
        && smooth_enough
        && (low_confidence_ssr || mirror_like || needs_accurate_reflection)
        && rt_resolution_lane(pixel, roughness, needs_accurate_reflection, mirror_like);

    var reflection = vec3<f32>(0.0);
    var confidence = 0.0;
    if (rt_allowed) {
        let rt_hit = trace_reflection(world + n * max(params.min_ray_offset, T_EPS), normalize(reflect(-v, n)), params.reflection_max_distance, probe);
        reflection = mix(probe, rt_hit.rgb, rt_hit.a) * fresnel * (1.0 - roughness * 0.5);
        // Confidence means an actual RT reflection hit. Do not mark a miss/probe
        // fallback as RT confidence, otherwise compose suppresses SSR with sky/probe.
        confidence = select(0.0, max(rt_hit.a, 0.30), rt_hit.a > 0.0);
    }

    // Alpha is RT confidence only. SSR/probe confidence remains in their own inputs.
    textureStore(effect_out, pixel, vec4<f32>(reflection, confidence));
}
