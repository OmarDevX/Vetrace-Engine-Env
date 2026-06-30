// Standalone screen-space reflections. Writes RGB reflection color and alpha confidence.
const SSR_STEPS: i32 = 28;
const SSR_STRIDE: f32 = 1.0;

struct SsrParams {
    inv_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    tex_size: vec2<f32>,
    max_distance: f32,
    thickness: f32,
    frame_number: u32,
    enabled: u32,
    _pad: vec2<u32>,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var normal_tex: texture_2d<f32>;
@group(0) @binding(2) var albedo_tex: texture_2d<f32>;
@group(0) @binding(3) var current_color_tex: texture_2d<f32>;
@group(0) @binding(4) var history_tex: texture_2d<f32>;
@group(0) @binding(5) var material_tex: texture_2d<u32>;
@group(0) @binding(6) var ssr_out: texture_storage_2d<rgba16float, write>;
@group(0) @binding(7) var<uniform> ssr: SsrParams;

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), depth, 1.0);
    let world = ssr.inv_view_proj * clip;
    return world.xyz / max(world.w, 1.0e-6);
}

fn project_to_uv(world: vec3<f32>) -> vec3<f32> {
    let clip = ssr.view_proj * vec4<f32>(world, 1.0);
    let ndc = clip.xyz / max(clip.w, 1.0e-6);
    return vec3<f32>(ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5), ndc.z);
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (ssr.enabled == 0u) { textureStore(ssr_out, pixel, vec4<f32>(0.0)); return; }
    let depth = textureLoad(depth_tex, pixel, 0).r;
    let albedo = textureLoad(albedo_tex, pixel, 0);
    if (depth >= 0.9999 || albedo.a <= 0.0) { textureStore(ssr_out, pixel, vec4<f32>(0.0)); return; }

    let mat = textureLoad(material_tex, pixel, 0);
    let roughness = clamp(f32(mat.g) / 255.0, 0.04, 1.0);
    let world = reconstruct_world(pixel, dims, depth);
    let n = unpack_normal(pixel);
    let v = normalize(ssr.camera_pos.xyz - world);
    let ray_dir = normalize(reflect(-v, n));
    let step_len = max(ssr.max_distance / f32(SSR_STEPS), SSR_STRIDE) * (0.35 + roughness);

    var hit_color = vec3<f32>(0.0);
    var confidence = 0.0;
    for (var i: i32 = 1; i <= SSR_STEPS; i = i + 1) {
        let t = min(f32(i) * step_len, ssr.max_distance);
        let sample_world = world + ray_dir * t;
        let proj = project_to_uv(sample_world);
        if (any(proj.xy < vec2<f32>(0.0)) || any(proj.xy > vec2<f32>(1.0)) || proj.z < 0.0 || proj.z > 1.0) { break; }
        let sp = clamp(vec2<i32>(proj.xy * vec2<f32>(dims)), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
        let sd = textureLoad(depth_tex, sp, 0).r;
        if (sd >= 0.9999) { continue; }
        let scene_world = reconstruct_world(sp, dims, sd);
        let depth_error = abs(length(scene_world - ssr.camera_pos.xyz) - length(sample_world - ssr.camera_pos.xyz));
        let normal_ok = max(dot(unpack_normal(sp), n), 0.0);
        if (depth_error < ssr.thickness + roughness * 0.12 && normal_ok > 0.25) {
            let edge = min(min(proj.x, 1.0 - proj.x), min(proj.y, 1.0 - proj.y));
            let fade = clamp(edge * 8.0, 0.0, 1.0) * (1.0 - f32(i) / f32(SSR_STEPS + 1));
            let current = textureLoad(current_color_tex, sp, 0).rgb;
            let history = textureLoad(history_tex, sp, 0).rgb;
            hit_color = mix(current, history, 0.2) * mix(vec3<f32>(1.0), albedo.rgb, roughness * 0.35);
            confidence = clamp(normal_ok * fade * (1.0 - roughness * 0.65), 0.0, 1.0);
            break;
        }
    }
    textureStore(ssr_out, pixel, vec4<f32>(hit_color, confidence));
}
