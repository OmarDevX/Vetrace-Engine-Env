// Standalone screen-space reflections. Writes RGB reflection color and alpha confidence.
struct SsrParams {
    inv_view_proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    prev_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    tex_size: vec2<f32>,
    max_distance: f32,
    thickness: f32,
    temporal_blend: f32,
    roughness_cutoff: f32,
    confidence_threshold: f32,
    stride: f32,
    max_steps: u32,
    frame_number: u32,
    enabled: u32,
    _pad: u32,
};

struct SceneCandidate {
    valid: bool,
    pixel: vec2<i32>,
    world: vec3<f32>,
    delta: f32,
    depth_error: f32,
    quality: f32,
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

fn project_with_vp(vp: mat4x4<f32>, world: vec3<f32>) -> vec3<f32> {
    let clip = vp * vec4<f32>(world, 1.0);
    let ndc = clip.xyz / max(clip.w, 1.0e-6);
    return vec3<f32>(ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5), ndc.z);
}

fn project_to_uv(world: vec3<f32>) -> vec3<f32> {
    return project_with_vp(ssr.view_proj, world);
}

fn project_to_prev_uv(world: vec3<f32>) -> vec3<f32> {
    return project_with_vp(ssr.prev_view_proj, world);
}

fn unpack_normal(pixel: vec2<i32>) -> vec3<f32> {
    return normalize(textureLoad(normal_tex, pixel, 0).xyz * 2.0 - vec3<f32>(1.0));
}

fn hash12(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn load_scene_color(pixel: vec2<i32>, dims: vec2<u32>) -> vec3<f32> {
    let clamped = clamp(pixel, vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
    let lit = textureLoad(current_color_tex, clamped, 0).rgb;
    let alb = textureLoad(albedo_tex, clamped, 0).rgb;
    let has_lit = any(lit > vec3<f32>(0.001));
    return select(alb, lit, has_lit);
}

fn gather_scene_color(center: vec2<i32>, dims: vec2<u32>) -> vec3<f32> {
    // Small bilateral-ish color gather at the reflected hit. This hides single-pixel
    // ray-march holes/cut lines without blurring the actual reflective surface.
    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    let center_depth = textureLoad(depth_tex, clamp(center, vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1)), 0).r;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let p = clamp(center + vec2<i32>(x, y), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
            let d = textureLoad(depth_tex, p, 0).r;
            if (d < 0.9999) {
                let depth_close = 1.0 - smoothstep(0.002, 0.030, abs(d - center_depth));
                let o = vec2<f32>(f32(x), f32(y));
                let w = depth_close / (1.0 + dot(o, o) * 0.75);
                sum = sum + load_scene_color(p, dims) * w;
                weight_sum = weight_sum + w;
            }
        }
    }
    if (weight_sum <= 1.0e-5) {
        return load_scene_color(center, dims);
    }
    return sum / weight_sum;
}

fn empty_candidate(pixel: vec2<i32>) -> SceneCandidate {
    return SceneCandidate(false, pixel, vec3<f32>(0.0), 0.0, 1.0e9, 0.0);
}

fn find_scene_candidate(
    proj: vec3<f32>,
    sample_world: vec3<f32>,
    origin_world: vec3<f32>,
    dims: vec2<u32>,
    step_len: f32,
    roughness: f32,
    mirror_like: bool,
) -> SceneCandidate {
    let base = clamp(vec2<i32>(proj.xy * vec2<f32>(dims)), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
    var best = empty_candidate(base);

    // A single exact depth pixel is too fragile for mirror SSR: the ray often
    // passes between pixels at object silhouettes, producing tiny ground-contact
    // strips even when the whole caster is visible. Search a small footprint
    // around the projected ray sample and accept the nearest plausible surface.
    let mirror_thickness_boost = select(1.0, 1.85, mirror_like);
    let ndc_thickness = (max(0.020, ssr.thickness * 0.16) + roughness * 0.050) * mirror_thickness_boost;
    let world_thickness = (max(ssr.thickness * 2.35, step_len * 3.0) + roughness * 0.45) * mirror_thickness_boost;
    // Mirror rays need a slightly smaller origin reject. A large reject fixes
    // self-hits, but it also erases nearby reflected geometry and makes the
    // reflection fade away from the contact region.
    let same_surface_reject = select(max(0.040, step_len * 0.75), max(0.025, step_len * 0.45), mirror_like);
    let footprint_radius = select(1, 2, mirror_like);

    for (var oy: i32 = -2; oy <= 2; oy = oy + 1) {
        if (abs(oy) > footprint_radius) { continue; }
        for (var ox: i32 = -2; ox <= 2; ox = ox + 1) {
            if (abs(ox) > footprint_radius) { continue; }
            let p = clamp(base + vec2<i32>(ox, oy), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
            let sd = textureLoad(depth_tex, p, 0).r;
            if (sd >= 0.9999) {
                continue;
            }

            let scene_world = reconstruct_world(p, dims, sd);
            if (length(scene_world - origin_world) < same_surface_reject) {
                continue;
            }

            let delta = proj.z - sd;
            let depth_error = abs(length(scene_world - ssr.camera_pos.xyz) - length(sample_world - ssr.camera_pos.xyz));
            let can_be_hit = delta >= -ndc_thickness && depth_error <= world_thickness;
            if (!can_be_hit) {
                continue;
            }

            let pixel_offset = vec2<f32>(f32(ox), f32(oy));
            let pixel_weight = 1.0 / (1.0 + dot(pixel_offset, pixel_offset) * 0.50);
            let ndc_quality = 1.0 - smoothstep(ndc_thickness * 0.60, ndc_thickness * 2.75, abs(delta));
            let world_quality = 1.0 - smoothstep(world_thickness * 0.45, world_thickness, depth_error);
            let behind_bonus = select(0.65, 1.0, delta >= 0.0);
            let q = max(0.0, ndc_quality) * max(0.0, world_quality) * pixel_weight * behind_bonus;
            if (q > best.quality) {
                best = SceneCandidate(true, p, scene_world, delta, depth_error, q);
            }
        }
    }
    return best;
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
    let metallic = f32(mat.r) / 255.0;
    let roughness = clamp(f32(mat.g) / 255.0, 0.0, 1.0);
    let feature_flags = mat.a & 0x0fu;
    let accurate_reflection = (feature_flags & 0x1u) != 0u;
    let mirror_like = roughness <= 0.055 && (metallic >= 0.75 || accurate_reflection);
    if (roughness > ssr.roughness_cutoff) { textureStore(ssr_out, pixel, vec4<f32>(0.0)); return; }
    let world = reconstruct_world(pixel, dims, depth);
    let n = unpack_normal(pixel);
    let v = normalize(ssr.camera_pos.xyz - world);
    let ray_dir = normalize(reflect(-v, n));
    if (dot(ray_dir, n) <= 0.001) {
        textureStore(ssr_out, pixel, vec4<f32>(0.0));
        return;
    }

    // Choose enough samples for the ray's projected screen length. The old code
    // marched by fixed world-space distance only, so a reflected cube could occupy
    // many pixels while the ray still sampled only a few of them. That caused the
    // reflection to collapse into a small strip near the contact point.
    let start_proj = project_to_uv(world + ray_dir * 0.060);
    let end_proj = project_to_uv(world + ray_dir * ssr.max_distance);
    let screen_span = length((end_proj.xy - start_proj.xy) * vec2<f32>(dims));
    let screen_step_scale = select(0.85, 1.20, mirror_like);
    let min_steps = select(24.0, 40.0, mirror_like);
    let desired_steps = i32(clamp(screen_span * screen_step_scale, min_steps, f32(ssr.max_steps)));
    let steps = max(desired_steps, 1);

    // Keep a small world-space floor so nearby contact reflection does not self-hit,
    // but drive most sampling by screen coverage instead of world stride. Mirror
    // surfaces get denser marching; otherwise long rays fade because they simply
    // step over the reflected object.
    let stride_scale = select(0.42, 0.25, mirror_like);
    let step_len = max(ssr.stride * stride_scale, ssr.max_distance / f32(max(steps, 1)));
    let jitter = hash12(vec2<f32>(pixel) + vec2<f32>(f32(ssr.frame_number & 1023u) * 0.37, f32(ssr.frame_number & 255u) * 1.73));
    let start_t_floor = select(0.050, 0.025, mirror_like);
    let start_t = max(start_t_floor, step_len * (0.10 + jitter * 0.35));
    let near_bias = 0.0010;
    let max_depth_thickness = max(0.020, ssr.thickness * 0.16) + roughness * 0.050;

    var hit_color = vec3<f32>(0.0);
    var confidence = 0.0;
    var prev_t = start_t;
    var prev_delta = -1.0;
    var had_prev = false;

    for (var i: i32 = 0; i < steps; i = i + 1) {
        let t = min(start_t + f32(i) * step_len, ssr.max_distance);
        let sample_world = world + ray_dir * t;
        let proj = project_to_uv(sample_world);
        if (any(proj.xy < vec2<f32>(0.0)) || any(proj.xy > vec2<f32>(1.0)) || proj.z < 0.0 || proj.z > 1.0) { break; }

        let sp = clamp(vec2<i32>(proj.xy * vec2<f32>(dims)), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
        let sd = textureLoad(depth_tex, sp, 0).r;
        if (sd >= 0.9999) {
            had_prev = false;
            prev_t = t;
            continue;
        }

        let scene_world = reconstruct_world(sp, dims, sd);
        let projected_delta = proj.z - sd;
        if (length(scene_world - world) < max(0.040, step_len * 0.75)) {
            prev_t = t;
            prev_delta = projected_delta;
            had_prev = true;
            continue;
        }

        let crossed_surface = had_prev && prev_delta < -near_bias && projected_delta >= -near_bias;
        var candidate = find_scene_candidate(proj, sample_world, world, dims, step_len, roughness, mirror_like);
        let candidate_threshold = select(0.035, 0.008, mirror_like);
        let candidate_hit = candidate.valid && candidate.quality > candidate_threshold;

        if (crossed_surface || candidate_hit) {
            var hit_t = t;
            var hit_px = candidate.pixel;
            var hit_world_for_reprojection = candidate.world;
            var hit_quality = max(candidate.quality, 0.35);

            if (crossed_surface) {
                var lo = prev_t;
                var hi = t;
                for (var b: i32 = 0; b < 5; b = b + 1) {
                    let mid = 0.5 * (lo + hi);
                    let mid_world = world + ray_dir * mid;
                    let mid_proj = project_to_uv(mid_world);
                    if (any(mid_proj.xy < vec2<f32>(0.0)) || any(mid_proj.xy > vec2<f32>(1.0)) || mid_proj.z < 0.0 || mid_proj.z > 1.0) {
                        hi = mid;
                        continue;
                    }
                    let mid_px = clamp(vec2<i32>(mid_proj.xy * vec2<f32>(dims)), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
                    let mid_depth = textureLoad(depth_tex, mid_px, 0).r;
                    let mid_delta = mid_proj.z - mid_depth;
                    if (mid_depth < 0.9999 && mid_delta >= -near_bias) {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                hit_t = hi;
                let refined_world = world + ray_dir * hit_t;
                let refined_proj = project_to_uv(refined_world);
                hit_px = clamp(vec2<i32>(refined_proj.xy * vec2<f32>(dims)), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
                let refined_depth = textureLoad(depth_tex, hit_px, 0).r;
                if (refined_depth < 0.9999) {
                    hit_world_for_reprojection = reconstruct_world(hit_px, dims, refined_depth);
                } else {
                    hit_world_for_reprojection = refined_world;
                }
                hit_quality = max(hit_quality, 0.70);
            }

            let hit_proj = project_to_uv(hit_world_for_reprojection);
            let hit_n = unpack_normal(hit_px);
            let facing = clamp(dot(hit_n, -ray_dir) * 0.5 + 0.5, 0.30, 1.0);
            let edge = min(min(hit_proj.x, 1.0 - hit_proj.x), min(hit_proj.y, 1.0 - hit_proj.y));
            // Do not use artistic distance fade on mirror-like SSR. It was the
            // reason a visible cube faded to only a contact strip: the ray hit was
            // valid, but confidence was attenuated by hit distance. Keep only true
            // validity fades: screen edge and depth quality.
            let edge_fade = select(smoothstep(0.008, 0.080, edge), smoothstep(0.002, 0.030, edge), mirror_like);
            let regular_distance_fade = 1.0 - smoothstep(ssr.max_distance * 0.55, ssr.max_distance, hit_t);
            let mirror_distance_fade = max(0.88, 1.0 - smoothstep(ssr.max_distance * 0.92, ssr.max_distance, hit_t));
            let distance_fade = select(regular_distance_fade, mirror_distance_fade, mirror_like);
            let depth_quality = select(select(hit_quality, max(hit_quality, 0.75), crossed_surface), max(hit_quality, 0.82), mirror_like);
            let fade = edge_fade * distance_fade * depth_quality;

            let current = gather_scene_color(hit_px, dims);
            let prev_proj = project_to_prev_uv(hit_world_for_reprojection);
            var history = vec3<f32>(0.0);
            var history_ok = false;
            if (all(prev_proj.xy >= vec2<f32>(0.0)) && all(prev_proj.xy <= vec2<f32>(1.0)) && prev_proj.z >= 0.0 && prev_proj.z <= 1.0) {
                let hp = clamp(vec2<i32>(prev_proj.xy * vec2<f32>(dims)), vec2<i32>(0), vec2<i32>(dims) - vec2<i32>(1));
                history = textureLoad(history_tex, hp, 0).rgb;
                history_ok = any(history > vec3<f32>(0.001));
            }
            let history_blend = select(0.0, min(ssr.temporal_blend, 0.14), history_ok);
            hit_color = mix(current, history, history_blend) * mix(vec3<f32>(1.0), albedo.rgb, roughness * 0.16);
            let raw_confidence = clamp(facing * fade * (1.0 - roughness * 0.65), 0.0, 1.0);
            let soft_threshold = ssr.confidence_threshold * 0.35;
            let regular_confidence = smoothstep(soft_threshold, max(soft_threshold + 0.16, ssr.confidence_threshold + 0.08), raw_confidence) * raw_confidence;
            // For mirrors, avoid double-fading: raw_confidence already contains
            // edge/depth validity. Applying another threshold curve makes the
            // reflection visibly fade even when the hit is valid.
            confidence = select(regular_confidence, raw_confidence, mirror_like);
            break;
        }

        prev_t = t;
        prev_delta = projected_delta;
        had_prev = true;
    }
    textureStore(ssr_out, pixel, vec4<f32>(hit_color, confidence));
}
