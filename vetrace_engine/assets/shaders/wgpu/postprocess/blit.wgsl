// === Post: Lighting + Bloom + Fog + Temporal DOF (wgpu 0.20-safe) ===

@group(0) @binding(0) var lin_samp: sampler;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var normal_tex: texture_2d<f32>;
@group(0) @binding(4) var depth_tex: texture_2d<f32>;
@group(0) @binding(5) var occluder_tex: texture_2d<f32>;
@group(0) @binding(6) var nearest_samp: sampler;

// --- NEW: history inputs for temporal DOF ---
@group(0) @binding(9) var history_tex: texture_2d<f32>;          // last frame color (filterable)
@group(0) @binding(10) var depth_history_tex: texture_2d<f32>;     // last frame depth (non-filterable OK)
@group(0) @binding(11) var normal_history_tex: texture_2d<f32>;    // last frame normals (non-filterable OK)
@group(0) @binding(12) var history_samp: sampler;                  // linear sampler for history_tex

const PI: f32 = 3.14159265;

struct LightUniform {
    dir: vec2<f32>, _pad: vec2<f32>,
    color: vec3<f32>, intensity: f32,
};
@group(0) @binding(7) var<uniform> light: LightUniform;

struct PostFxUniforms {
    dof_enabled: u32,
    dof_manual: u32,
    dof_show_focus: u32,
    _dof_pad: u32,
    dof_focal_depth: f32,
    dof_focal_length: f32,
    dof_fstop: f32,
    dof_coc: f32,
    dof_ndof_start: f32,
    dof_ndof_dist: f32,
    dof_fdof_start: f32,
    dof_fdof_dist: f32,
    dof_max_blur: f32,
    dof_threshold: f32,
    dof_gain: f32,
    dof_bias: f32,
    dof_fringe: f32,
    dof_namount: f32,
    dof_samples: u32,
    dof_rings: u32,
    dof_noise: u32,
    dof_vignetting: u32,
    dof_autofocus: u32,
    dof_depth_blur: u32,
    dof_vignout: f32,
    dof_vignin: f32,
    dof_vignfade: f32,
    dof_focus_x: f32,
    dof_focus_y: f32,
    dof_db_size: f32,
    dof_feather: f32,
    dof_pentagon: u32,
    _dof_pad1: u32,
    z_near: f32,
    z_far: f32,
    bloom_enabled: u32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    bloom_spread: f32,
    bloom_iterations: u32,
    exposure: f32,
    auto_exposure: u32,
    sky_occlusion: f32,
    fog_density: f32,
    fog_color_r: f32,
    fog_color_g: f32,
    fog_color_b: f32,
    history_clamp_k: f32,
    temporal_blend: f32,
    gi_temporal_blend: f32,
    _pad0: u32, _pad1: u32, _pad2: u32, _pad3: u32,
};
@group(0) @binding(3) var<uniform> postfx: PostFxUniforms;

// --- NEW: minimal camera/jitter for reprojection ---
struct Params {
    camera_pos: vec4<f32>,
    inv_view_proj: mat4x4<f32>,  // built with CURRENT jitter
    prev_view_proj: mat4x4<f32>, // built with PREVIOUS jitter
    taa_jitter: vec2<f32>,       // current jitter (UV)
    prev_taa_jitter: vec2<f32>,  // previous jitter (UV)
    tex_size: vec2<f32>,         // source texture size
    sharpness: f32,
    _pad: f32,
};
@group(0) @binding(13) var<uniform> params: Params;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn luminance(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0), vec2<f32>( 1.0,  1.0)
    );
    var uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 1.0)
    );
    var out: VsOut;
    out.pos = vec4<f32>(positions[vi], 0.0, 1.0);
    out.uv  = uvs[vi];
    return out;
}

// ---- helpers ----
fn linearize_depth(depth01: f32, zn: f32, zf: f32) -> f32 {
    // reverse of projection (assuming standard depth)
    return (zn * zf) / max(zf - depth01 * (zf - zn), 1e-6);
}

fn rand(coord: vec2<f32>, dims: vec2<f32>) -> vec2<f32> {
    var noise_x = (fract(1.0 - coord.x * (dims.x * 0.5)) * 0.25
        + fract(coord.y * (dims.y * 0.5)) * 0.75) * 2.0 - 1.0;
    var noise_y = (fract(1.0 - coord.x * (dims.x * 0.5)) * 0.75
        + fract(coord.y * (dims.y * 0.5)) * 0.25) * 2.0 - 1.0;
    if (postfx.dof_noise != 0u) {
        noise_x = fract(sin(dot(coord, vec2<f32>(12.9898, 78.233))) * 43758.5453) * 2.0 - 1.0;
        noise_y = fract(sin(dot(coord, vec2<f32>(12.9898, 78.233) * 2.0)) * 43758.5453) * 2.0 - 1.0;
    }
    return vec2<f32>(noise_x, noise_y);
}

fn penta(coords: vec2<f32>) -> f32 {
    let scale = f32(postfx.dof_rings) - 1.3;
    let HS0 = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    let HS1 = vec4<f32>(0.309016994, 0.951056516, 0.0, 1.0);
    let HS2 = vec4<f32>(-0.809016994, 0.587785252, 0.0, 1.0);
    let HS3 = vec4<f32>(-0.809016994, -0.587785252, 0.0, 1.0);
    let HS4 = vec4<f32>(0.309016994, -0.951056516, 0.0, 1.0);
    let HS5 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    let one = vec4<f32>(1.0);
    let P = vec4<f32>(coords, vec2<f32>(scale, scale));
    var dist = vec4<f32>(0.0);
    var inorout = -4.0;
    dist.x = dot(P, HS0);
    dist.y = dot(P, HS1);
    dist.z = dot(P, HS2);
    dist.w = dot(P, HS3);
    let feather = vec4<f32>(
        postfx.dof_feather,
        postfx.dof_feather,
        postfx.dof_feather,
        postfx.dof_feather,
    );
    dist = smoothstep(-feather, feather, dist);
    inorout += dot(dist, one);
    dist.x = dot(P, HS4);
    dist.y = HS5.w - abs(P.z);
    dist = smoothstep(-feather, feather, dist);
    inorout += dist.x;
    return clamp(inorout, 0.0, 1.0);
}

fn bdepth(coords: vec2<f32>, texel: vec2<f32>) -> f32 {
    var d = 0.0;
    let wh = texel * postfx.dof_db_size;
    let offsets = array<vec2<f32>, 9>(
        vec2<f32>(-wh.x, -wh.y), vec2<f32>(0.0, -wh.y), vec2<f32>(wh.x, -wh.y),
        vec2<f32>(-wh.x, 0.0),  vec2<f32>(0.0, 0.0),  vec2<f32>(wh.x, 0.0),
        vec2<f32>(-wh.x, wh.y), vec2<f32>(0.0, wh.y), vec2<f32>(wh.x, wh.y)
    );
    let kernel = array<f32, 9>(
        1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0,
        2.0 / 16.0, 4.0 / 16.0, 2.0 / 16.0,
        1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0
    );
    // Unrolled manual accumulation to satisfy WGSL constant-indexing rules
    d += textureSample(depth_tex, nearest_samp, coords + offsets[0]).r * kernel[0];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[1]).r * kernel[1];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[2]).r * kernel[2];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[3]).r * kernel[3];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[4]).r * kernel[4];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[5]).r * kernel[5];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[6]).r * kernel[6];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[7]).r * kernel[7];
    d += textureSample(depth_tex, nearest_samp, coords + offsets[8]).r * kernel[8];
    return d;
}

fn vignette(uv: vec2<f32>) -> f32 {
    var dist = distance(uv, vec2<f32>(0.5, 0.5));
    dist = smoothstep(postfx.dof_vignout + (postfx.dof_fstop / postfx.dof_vignfade),
                      postfx.dof_vignin + (postfx.dof_fstop / postfx.dof_vignfade), dist);
    return clamp(dist, 0.0, 1.0);
}

fn color_sample(coords: vec2<f32>, blur: f32, texel: vec2<f32>) -> vec3<f32> {
    var col = vec3<f32>(0.0);
    col.r = textureSample(tex, lin_samp, coords + vec2<f32>(0.0, 1.0) * texel * postfx.dof_fringe * blur).r;
    col.g = textureSample(tex, lin_samp, coords + vec2<f32>(-0.866, -0.5) * texel * postfx.dof_fringe * blur).g;
    col.b = textureSample(tex, lin_samp, coords + vec2<f32>(0.866, -0.5) * texel * postfx.dof_fringe * blur).b;
    let lum = dot(col, vec3<f32>(0.299, 0.587, 0.114));
    let thresh = max((lum - postfx.dof_threshold) * postfx.dof_gain, 0.0);
    return col + mix(vec3<f32>(0.0), col, thresh * blur);
}

fn debug_focus(col: vec3<f32>, blur: f32, depth: f32) -> vec3<f32> {
    let edge = 0.002 * depth;
    let m = clamp(smoothstep(0.0, edge, blur), 0.0, 1.0);
    let e = clamp(smoothstep(1.0 - edge, 1.0, blur), 0.0, 1.0);
    var c = mix(col, vec3<f32>(1.0, 0.5, 0.0), (1.0 - m) * 0.6);
    c = mix(c, vec3<f32>(0.0, 0.5, 1.0), ((1.0 - e) - (1.0 - m)) * 0.2);
    return c;
}

fn get_view_dir(uv: vec2<f32>) -> vec3<f32> {
    // reconstruct a world ray direction using inv_view_proj and current jitter
    let uvj = uv * 2.0 - vec2<f32>(1.0) + vec2<f32>(params.taa_jitter * 2.0);
    let p0  = vec4<f32>(uvj, 0.0, 1.0);
    let p1  = vec4<f32>(uvj, 1.0, 1.0);
    let w0  = params.inv_view_proj * p0;
    let w1  = params.inv_view_proj * p1;
    let a   = w0.xyz / w0.w;
    let b   = w1.xyz / w1.w;
    return normalize(b - a);
}

fn reproject_prev_uv(cur_uv: vec2<f32>, cur_depth01: f32) -> vec2<f32> {
    // approximate world pos along the view ray using linearized depth
    let z_view = linearize_depth(cur_depth01, postfx.z_near, postfx.z_far);
    let dir = get_view_dir(cur_uv);
    let wpos = params.camera_pos.xyz + dir * z_view;

    let prev_cs = params.prev_view_proj * vec4<f32>(wpos, 1.0);
    let prev_ndc = prev_cs.xy / prev_cs.w;
    var prev_uv = prev_ndc * 0.5 + vec2<f32>(0.5);
    // compensate jitter delta (prev built with prev jitter)
    prev_uv += (params.prev_taa_jitter - params.taa_jitter);
    return prev_uv;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let cur_col = textureSample(tex, lin_samp, in.uv);
    let texel = vec2<f32>(1.0) / params.tex_size;
    let c = cur_col.rgb;
    let n = textureSample(tex, lin_samp, in.uv + vec2<f32>(texel.x, 0.0)).rgb;
    let s = textureSample(tex, lin_samp, in.uv - vec2<f32>(texel.x, 0.0)).rgb;
    let e = textureSample(tex, lin_samp, in.uv + vec2<f32>(0.0, texel.y)).rgb;
    let w = textureSample(tex, lin_samp, in.uv - vec2<f32>(0.0, texel.y)).rgb;
    let avg = (n + s + e + w) * 0.25;
    let sharpened = c + params.sharpness * (c - avg);
    var color = vec4<f32>(sharpened, cur_col.a);

    // --------- simple 2D “godray” shadow (kept as-is) ----------
    if (light.intensity > 0.0) {
        let occ = textureSample(occluder_tex, lin_samp, in.uv).r;
        var shadow = 1.0;
        if (occ <= 0.5) {
            var s_uv = in.uv;
            let dir = normalize(light.dir);
            let step = dir / f32(32);
            var hit = false;
            var dist = 0.0;
            for (var i: i32 = 0; i < 32; i = i + 1) {
                s_uv -= step;
                if (any(s_uv < vec2<f32>(0.0)) || any(s_uv > vec2<f32>(1.0))) { break; }
                if (textureSample(occluder_tex, lin_samp, s_uv).r > 0.5) {
                    hit = true; dist = f32(i) / 32.0; break;
                }
            }
            if (hit) {
                let dims = vec2<f32>(textureDimensions(occluder_tex));
                let px = 1.0 / dims;
                let radius = dist * 8.0;
                var offsets = array<vec2<f32>, 4>(
                    vec2<f32>(-1.0, 0.0), vec2<f32>(1.0, 0.0),
                    vec2<f32>(0.0, -1.0), vec2<f32>(0.0, 1.0)
                );
                var vis = 0.0;
                for (var j: i32 = 0; j < 4; j = j + 1) {
                    var o_uv = in.uv + offsets[j] * px * radius;
                    var t_uv = o_uv;
                    var blocked = false;
                    for (var k: i32 = 0; k < 32; k = k + 1) {
                        t_uv -= step;
                        if (any(t_uv < vec2<f32>(0.0)) || any(t_uv > vec2<f32>(1.0))) { break; }
                        if (textureSample(occluder_tex, lin_samp, t_uv).r > 0.5) { blocked = true; break; }
                    }
                    vis += select(1.0, 0.0, blocked);
                }
                shadow = vis / 5.0;
            }
        }
        let lighting = 0.2 + shadow * light.intensity;
        color = vec4<f32>(color.rgb * light.color * lighting, color.a);
    }

    // ================== BOKEH DOF ==================
    if (postfx.dof_enabled != 0u) {
        let dims = vec2<f32>(textureDimensions(tex));
        let texel = vec2<f32>(1.0) / dims;
        var depth01 = textureSample(depth_tex, nearest_samp, in.uv).r;
        if (postfx.dof_depth_blur != 0u) {
            depth01 = bdepth(in.uv, texel);
        }
        let depth = linearize_depth(depth01, postfx.z_near, postfx.z_far);
        var f_depth = postfx.dof_focal_depth;
        if (postfx.dof_autofocus != 0u) {
            let fd = textureSample(depth_tex, nearest_samp, vec2<f32>(postfx.dof_focus_x, postfx.dof_focus_y)).r;
            f_depth = linearize_depth(fd, postfx.z_near, postfx.z_far);
        }
        var blur: f32;
        if (postfx.dof_manual != 0u) {
            let a = depth - f_depth;
            let b = (a - postfx.dof_fdof_start) / postfx.dof_fdof_dist;
            let c = (-a - postfx.dof_ndof_start) / postfx.dof_ndof_dist;
            blur = select(c, b, a > 0.0);
        } else {
            let f = postfx.dof_focal_length;
            let d = f_depth * 1000.0;
            let o = depth * 1000.0;
            let a = (o * f) / (o - f);
            let b = (d * f) / (d - f);
            let c = (d - f) / (d * postfx.dof_fstop * postfx.dof_coc);
            blur = abs(a - b) * c;
        }
        blur = clamp(blur, 0.0, 1.0);

        let noise = rand(in.uv, dims) * postfx.dof_namount * blur;
        let w = texel.x * blur * postfx.dof_max_blur + noise.x;
        let h = texel.y * blur * postfx.dof_max_blur + noise.y;

        var col: vec3<f32>;
        if (blur < 0.05) {
            // Minimal blur: just fetch the source color without chromatic effects
            col = color.rgb;
        } else {
            // Full bokeh sampling: include chromatic fringe and highlight boost
            col = color_sample(in.uv, blur, texel);
            var s = 1.0;
            let samples = i32(postfx.dof_samples);
            let rings = i32(postfx.dof_rings);
            for (var i: i32 = 1; i <= rings; i = i + 1) {
                let ring_samples = i * samples;
                let step = PI * 2.0 / f32(ring_samples);
                for (var j: i32 = 0; j < ring_samples; j = j + 1) {
                    let pw = cos(f32(j) * step) * f32(i);
                    let ph = sin(f32(j) * step) * f32(i);
                    var p = 1.0;
                    if (postfx.dof_pentagon != 0u) {
                        p = penta(vec2<f32>(pw, ph));
                    }
                    col += color_sample(in.uv + vec2<f32>(pw * w, ph * h), blur, texel) *
                        mix(1.0, f32(i) / f32(rings), postfx.dof_bias) * p;
                    s += 1.0 * mix(1.0, f32(i) / f32(rings), postfx.dof_bias) * p;
                }
            }
            col = col / s;
        }
        if (postfx.dof_show_focus != 0u) {
            col = debug_focus(col, blur, depth);
        }
        if (postfx.dof_vignetting != 0u) {
            col *= vignette(in.uv);
        }
        color = vec4<f32>(col, color.a);
    }
    // ================== END BOKEH DOF ==================

    // ---------- Bloom (unchanged) ----------
    if (postfx.bloom_enabled != 0u) {
        let dims = vec2<f32>(textureDimensions(tex));
        let px = postfx.bloom_spread / dims;
        let thr = vec3<f32>(postfx.bloom_threshold);
        let iter = i32(postfx.bloom_iterations);
        var glow = vec3<f32>(0.0);
        var total = 0.0;
        for (var i: i32 = -iter; i <= iter; i = i + 1) {
            for (var j: i32 = -iter; j <= iter; j = j + 1) {
                let offset = vec2<f32>(f32(i), f32(j));
                let uv = in.uv + px * offset;
                let sampc = textureSample(tex, lin_samp, uv).rgb;
                let diff = max(sampc - thr, vec3<f32>(0.0));
                let weight = pow(max(1.0 - length(offset) / f32(iter), 0.0), 2.0);
                glow += diff * weight;
                total += weight;
            }
        }
        if (total > 0.0) {
            glow = glow / total;
            color = vec4<f32>(color.rgb + glow * postfx.bloom_intensity, color.a);
        }
    }

    // ---------- Fog (unchanged) ----------
    if (postfx.fog_density > 0.0) {
        let depth01 = textureSample(depth_tex, nearest_samp, in.uv).r;
        let fog = 1.0 - exp(-depth01 * postfx.fog_density);
        let fog_color = vec4<f32>(postfx.fog_color_r, postfx.fog_color_g, postfx.fog_color_b, color.a);
        color = mix(color, fog_color, fog);
    }

    // ---------- Exposure (unchanged) ----------
    var exp_scale = postfx.exposure;
    if (postfx.auto_exposure != 0u) {
        let dims = vec2<f32>(textureDimensions(tex));
        let max_dim = max(dims.x, dims.y);
        let lod = floor(log2(max_dim));
        let avg = luminance(textureSampleLevel(tex, lin_samp, vec2<f32>(0.5, 0.5), lod).rgb);
        let targetc = 0.5;
        let ratio = clamp(targetc / max(avg, 0.001), 0.25, 4.0);
        exp_scale = postfx.exposure * ratio;
    }
    color = vec4<f32>(color.rgb * exp_scale, color.a);
    return color;
}
