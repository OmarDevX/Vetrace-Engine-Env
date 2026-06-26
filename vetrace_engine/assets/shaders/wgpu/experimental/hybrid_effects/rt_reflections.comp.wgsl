// EXPERIMENTAL/FUTURE: moved out of the active hybrid shader directory because Rust does not wire this shader into a pipeline yet. See docs/SHADER_ARCHITECTURE.md.
struct RtEffectParams {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    dir_light_dir: vec4<f32>,
    dir_light_color: vec4<f32>,
    enabled: u32,
    // Reflection quality/resolution gate supplied by the renderer:
    // 0 = performance (quarter-rate RT), 1 = default (half-rate RT), 2 = mirror/full-rate.
    mode: u32,
    _pad: vec2<u32>,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var material_tex: texture_2d<u32>;
@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var roughness_tex: texture_2d<f32>;
@group(0) @binding(5) var object_id_tex: texture_2d<u32>;
@group(0) @binding(6) var effect_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> rt_params: RtEffectParams;

const ROUGH_PROBE_ONLY: f32 = 0.60;
const ROUGH_RT_ALLOWED: f32 = 0.25;
const SSR_STEPS: i32 = 18;
const SSR_STRIDE: f32 = 9.0;
const SSR_THICKNESS: f32 = 0.015;


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

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var clip = vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    var world = rt_params.inv_view_proj * clip;
    world = world / world.w;
    return world.xyz;
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
    let rt_allowed = smooth_enough && important_object && insufficient_fallback && rt_resolution_lane(pixel, roughness);

    var reflection = mix(probe * fresnel, ssr_color, ssr.a);
    var confidence = max(ssr.a, 0.25);
    if (rt_allowed) {
        // Placeholder for the expensive RT ray result; keep confidence in alpha so
        // the compositor can distinguish sparse RT from cheap SSR/probe fallback.
        let rt_estimate = mix(rt_params.dir_light_color.rgb, albedo, roughness) * fresnel * (1.0 - roughness);
        reflection = rt_estimate;
        confidence = 1.0 - roughness;
    } else if (mid_roughness_blend) {
        reflection = mix(probe * fresnel, ssr_color, clamp(ssr.a * 0.75, 0.0, 0.5));
        confidence = max(confidence, 0.35);
    }

    textureStore(effect_out, pixel, vec4<f32>(reflection, confidence));
}
