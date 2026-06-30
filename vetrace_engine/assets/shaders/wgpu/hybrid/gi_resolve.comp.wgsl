const GI_RESOLVE_METHOD_OFF: u32 = 0u;
const GI_RESOLVE_METHOD_BAKED_LIGHTMAP: u32 = 1u;
const GI_RESOLVE_METHOD_LIGHT_PROBES: u32 = 2u;
const GI_RESOLVE_METHOD_SDFGI: u32 = 3u;
const GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE: u32 = 4u;

struct GiResolveParams {
    selected_method: u32,
    frame_number: u32,
    debug_flags: u32,
    _pad0: u32,
    temporal_blend: f32,
    baked_blend: f32,
    probe_blend: f32,
    sdfgi_blend: f32,
    rtgi_blend: f32,
    _pad1: vec3<f32>,
    inv_view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var gbuf_albedo: texture_2d<f32>;
@group(0) @binding(2) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(3) var lightmap_tex: texture_2d<f32>;
@group(0) @binding(4) var sdfgi_radiance: texture_3d<f32>;
@group(0) @binding(5) var rtgi_radiance: texture_2d<f32>;
@group(0) @binding(6) var gi_history: texture_2d<f32>;
@group(0) @binding(7) var gi_buffer: texture_storage_2d<rgba16float, write>;
@group(0) @binding(8) var<uniform> params: GiResolveParams;

fn resolved_surface(pixel: vec2<i32>) -> bool {
    return textureLoad(depth_tex, pixel, 0).r < 0.9999 && textureLoad(gbuf_albedo, pixel, 0).a > 0.0;
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(gbuf_normal, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn resolve_light_probe(pixel: vec2<i32>) -> vec3<f32> {
    let n = unpack_normal(pixel);
    return vec3<f32>(0.42, 0.50, 0.62) * (0.35 + 0.65 * clamp(n.y * 0.5 + 0.5, 0.0, 1.0));
}

fn reconstruct_world(pixel: vec2<i32>, dims: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var world = params.inv_view_proj * vec4<f32>(uv * 2.0 - vec2<f32>(1.0), depth, 1.0);
    return (world / world.w).xyz;
}

fn load_rtgi_denoised(pixel: vec2<i32>, dims: vec2<u32>) -> vec3<f32> {
    let center_n = unpack_normal(pixel);
    let center_depth = textureLoad(depth_tex, pixel, 0).r;
    var sum = textureLoad(rtgi_radiance, pixel, 0).rgb;
    var wsum = 1.0;
    for (var oy = -1; oy <= 1; oy = oy + 1) {
        for (var ox = -1; ox <= 1; ox = ox + 1) {
            if (ox == 0 && oy == 0) { continue; }
            let q = pixel + vec2<i32>(ox, oy);
            if (q.x < 0 || q.y < 0 || q.x >= i32(dims.x) || q.y >= i32(dims.y) || !resolved_surface(q)) { continue; }
            let ndot = max(dot(center_n, unpack_normal(q)), 0.0);
            let d = abs(textureLoad(depth_tex, q, 0).r - center_depth);
            let w = ndot * ndot * exp(-d * 96.0);
            sum = sum + textureLoad(rtgi_radiance, q, 0).rgb * w;
            wsum = wsum + w;
        }
    }
    return sum / max(wsum, 1.0e-4);
}

fn reproject_history(pixel: vec2<i32>, dims: vec2<u32>) -> vec3<f32> {
    let world = reconstruct_world(pixel, dims, textureLoad(depth_tex, pixel, 0).r);
    let prev_clip = params.prev_view_proj * vec4<f32>(world, 1.0);
    let prev_ndc = prev_clip.xyz / max(prev_clip.w, 1.0e-5);
    let prev_uv = prev_ndc.xy * 0.5 + vec2<f32>(0.5);
    let prev_px = vec2<i32>(prev_uv * vec2<f32>(dims));
    if (prev_px.x < 0 || prev_px.y < 0 || prev_px.x >= i32(dims.x) || prev_px.y >= i32(dims.y)) { return textureLoad(gi_history, pixel, 0).rgb; }
    return textureLoad(gi_history, prev_px, 0).rgb;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let pixel = vec2<i32>(id.xy);
    if (!resolved_surface(pixel) || params.selected_method == GI_RESOLVE_METHOD_OFF) {
        textureStore(gi_buffer, pixel, vec4<f32>(0.0, 0.0, 0.0, 1.0));
        return;
    }

    let uv = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / vec2<f32>(dims);
    var gi = vec3<f32>(0.0);
    if (params.selected_method == GI_RESOLVE_METHOD_BAKED_LIGHTMAP) {
        gi = textureLoad(lightmap_tex, pixel, 0).rgb * params.baked_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_LIGHT_PROBES) {
        gi = resolve_light_probe(pixel) * params.probe_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_SDFGI) {
        let sdf_dims = textureDimensions(sdfgi_radiance);
        let coord = vec3<i32>(vec3<u32>(id.x % sdf_dims.x, id.y % sdf_dims.y, (params.frame_number / 4u) % sdf_dims.z));
        gi = textureLoad(sdfgi_radiance, coord, 0).rgb * params.sdfgi_blend;
    } else if (params.selected_method == GI_RESOLVE_METHOD_RTGI_ONE_BOUNCE) {
        gi = load_rtgi_denoised(pixel, dims) * params.rtgi_blend;
    }

    if (params.frame_number > 0u && params.temporal_blend > 0.0) {
        let history = reproject_history(pixel, dims);
        let lo = max(vec3<f32>(0.0), gi - vec3<f32>(params.rtgi_blend + 0.25));
        let hi = gi + vec3<f32>(params.rtgi_blend + 0.25);
        gi = mix(gi, clamp(history, lo, hi), clamp(params.temporal_blend, 0.0, 0.96));
    }
    textureStore(gi_buffer, pixel, vec4<f32>(max(gi, vec3<f32>(0.0)), 1.0));
}
