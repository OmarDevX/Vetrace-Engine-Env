struct AmbientOcclusionParams {
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    tex_size: vec2<f32>,
    radius: f32,
    intensity: f32,
    method: u32,
    frame_number: u32,
    temporal_enabled: u32,
    _pad: u32,
};

const AO_METHOD_SSAO: u32 = 1u;
const AO_METHOD_GTAO: u32 = 2u;

@group(0) @binding(0) var depth_tex: texture_2d<f32>;
@group(0) @binding(1) var gbuf_normal: texture_2d<f32>;
@group(0) @binding(2) var ao_history: texture_2d<f32>;
@group(0) @binding(3) var ao_out: texture_storage_2d<r16float, write>;
@group(0) @binding(4) var<uniform> params: AmbientOcclusionParams;

fn reconstruct_world(px: vec2<i32>, depth01: f32, dims: vec2<u32>) -> vec3<f32> {
    let uv = (vec2<f32>(px) + vec2<f32>(0.5)) / vec2<f32>(dims);
    let clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), depth01, 1.0);
    let world_h = params.inv_view_proj * clip;
    return world_h.xyz / max(world_h.w, 1e-6);
}

fn decode_normal(px: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(gbuf_normal, px, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn interleaved_gradient_noise(p: vec2<u32>) -> f32 {
    let n = p.x * 1973u + p.y * 9277u + params.frame_number * 26699u + 89173u;
    return f32((n ^ (n << 13u)) * 15731u + 789221u & 0x00ffffffu) / 16777215.0;
}

fn sample_occlusion(px: vec2<i32>, world: vec3<f32>, normal: vec3<f32>, offset: vec2<i32>, dims: vec2<u32>) -> f32 {
    let sp = clamp(px + offset, vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
    let sd = textureLoad(depth_tex, sp, 0).r;
    if (sd >= 0.9999) { return 0.0; }
    let sw = reconstruct_world(sp, sd, dims);
    let delta = sw - world;
    let dist = length(delta);
    if (dist <= 1.0e-4 || dist >= params.radius) { return 0.0; }

    // Contact AO only: accept samples in the receiver hemisphere, but avoid
    // turning AO into a thick distant shadow. This keeps corners readable while
    // stopping the heavy dark halo seen at distance.
    let dir = delta / max(dist, 1.0e-5);
    let hemisphere = clamp(dot(normal, dir), 0.0, 1.0);
    let near_reject = smoothstep(0.025, max(0.06, params.radius * 0.08), dist);
    let range = 1.0 - smoothstep(params.radius * 0.28, params.radius, dist);
    let contact = smoothstep(0.05, 0.55, hemisphere);
    return contact * range * near_reject;
}

fn ssao(px: vec2<i32>, world: vec3<f32>, normal: vec3<f32>, dims: vec2<u32>) -> f32 {
    let jitter = interleaved_gradient_noise(vec2<u32>(px));
    var occ = 0.0;
    var weight_sum = 0.0;
    for (var i = 0; i < 16; i = i + 1) {
        let fi = f32(i);
        let a = (fi + jitter) * 2.3999632;
        let ring = sqrt((fi + 0.5) / 16.0);
        let r = mix(1.0, 8.0, ring);
        let off = vec2<i32>(round(vec2<f32>(cos(a), sin(a)) * r));
        let w = mix(1.0, 0.35, ring);
        occ = occ + sample_occlusion(px, world, normal, off, dims) * w;
        weight_sum = weight_sum + w;
    }
    return clamp(1.0 - occ * params.intensity / max(weight_sum, 1.0e-4), 0.0, 1.0);
}

fn gtao_direction(d: i32) -> vec2<i32> {
    if (d == 0) {
        return vec2<i32>(1, 0);
    }
    if (d == 1) {
        return vec2<i32>(0, 1);
    }
    if (d == 2) {
        return vec2<i32>(1, 1);
    }
    return vec2<i32>(1, -1);
}

fn gtao(px: vec2<i32>, world: vec3<f32>, normal: vec3<f32>, dims: vec2<u32>) -> f32 {
    var horizon = 0.0;
    for (var d = 0; d < 4; d = d + 1) {
        let dir = gtao_direction(d);
        var dir_occ = 0.0;
        for (var s = 1; s <= 4; s = s + 1) {
            dir_occ = max(dir_occ, sample_occlusion(px, world, normal, dir * i32(s * 2), dims));
            dir_occ = max(dir_occ, sample_occlusion(px, world, normal, -dir * i32(s * 2), dims));
        }
        horizon = horizon + dir_occ;
    }
    return clamp(1.0 - horizon * params.intensity * 0.28, 0.0, 1.0);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(depth_tex);
    if (id.x >= dims.x || id.y >= dims.y) { return; }
    let px = vec2<i32>(id.xy);
    let depth01 = textureLoad(depth_tex, px, 0).r;
    if (depth01 >= 0.9999) {
        textureStore(ao_out, px, vec4<f32>(1.0, 0.0, 0.0, 1.0));
        return;
    }
    let world = reconstruct_world(px, depth01, dims);
    let normal = decode_normal(px);
    var ao = 1.0;
    if (params.method == AO_METHOD_SSAO) {
        ao = ssao(px, world, normal, dims);
    } else if (params.method == AO_METHOD_GTAO) {
        ao = gtao(px, world, normal, dims);
    }
    if (params.temporal_enabled != 0u) {
        let history = textureLoad(ao_history, px, 0).r;
        ao = mix(ao, history, 0.35);
    }
    textureStore(ao_out, px, vec4<f32>(ao, 0.0, 0.0, 1.0));
}
