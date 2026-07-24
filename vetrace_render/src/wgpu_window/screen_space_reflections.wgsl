// Generic forward-renderer SSR. Screen-visible hits are blended over the
// existing scene, which already contains reflection-probe fallback.
struct CustomPostProcessUniform {
    p0: vec4<f32>, // intensity, max distance, thickness, stride
    p1: vec4<f32>, // max steps, edge fade, start distance, enabled
    p2: vec4<f32>, // origin bias, distance-fade start, normal rejection, max confidence
    p3: vec4<f32>, // temporal enabled, history weight, clamp expansion, disocclusion threshold
    p4: vec4<f32>,
    p5: vec4<f32>,
    p6: vec4<f32>,
    p7: vec4<f32>,
    screen_time: vec4<f32>,
    info: vec4<f32>,
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    camera_position: vec4<f32>,
    camera_forward: vec4<f32>,
    previous_view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var scene_color: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@group(0) @binding(3) var<uniform> post: CustomPostProcessUniform;
@group(0) @binding(4) var history_color: texture_2d<f32>;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn fullscreen_triangle_position(vertex_index: u32) -> vec2<f32> {
    var p = vec2<f32>(-1.0, -3.0);
    if (vertex_index == 1u) { p = vec2<f32>(3.0, 1.0); }
    if (vertex_index == 2u) { p = vec2<f32>(-1.0, 1.0); }
    return p;
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VsOut {
    var out: VsOut;
    let p = fullscreen_triangle_position(vertex_index);
    out.position = vec4<f32>(p, 0.0, 1.0);
    out.uv = p * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

fn dimensions_i() -> vec2<i32> { return vec2<i32>(textureDimensions(scene_depth)); }
fn clamp_pixel(pixel: vec2<i32>) -> vec2<i32> {
    return clamp(pixel, vec2<i32>(0), dimensions_i() - vec2<i32>(1));
}
fn depth_at(pixel: vec2<i32>) -> f32 { return textureLoad(scene_depth, clamp_pixel(pixel), 0); }

fn world_from_depth(pixel: vec2<i32>, depth: f32) -> vec3<f32> {
    let dims = vec2<f32>(textureDimensions(scene_depth));
    let uv = (vec2<f32>(clamp_pixel(pixel)) + vec2<f32>(0.5)) / dims;
    let clip = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), depth, 1.0);
    let world = post.inverse_view_proj * clip;
    let safe_w = select(1.0e-6, world.w, abs(world.w) > 1.0e-6);
    return world.xyz / safe_w;
}

fn project_world(world: vec3<f32>) -> vec3<f32> {
    let clip = post.view_proj * vec4<f32>(world, 1.0);
    let safe_w = select(1.0e-6, clip.w, abs(clip.w) > 1.0e-6);
    let ndc = clip.xyz / safe_w;
    return vec3<f32>(ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5), ndc.z);
}

fn shorter_vector(a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    return select(b, a, dot(a, a) < dot(b, b));
}

fn reconstructed_normal(pixel: vec2<i32>, center_world: vec3<f32>) -> vec3<f32> {
    let left = world_from_depth(pixel + vec2<i32>(-1, 0), depth_at(pixel + vec2<i32>(-1, 0)));
    let right = world_from_depth(pixel + vec2<i32>(1, 0), depth_at(pixel + vec2<i32>(1, 0)));
    let up = world_from_depth(pixel + vec2<i32>(0, -1), depth_at(pixel + vec2<i32>(0, -1)));
    let down = world_from_depth(pixel + vec2<i32>(0, 1), depth_at(pixel + vec2<i32>(0, 1)));
    let dx = shorter_vector(right - center_world, center_world - left);
    let dy = shorter_vector(down - center_world, center_world - up);
    let raw_normal = cross(dx, dy);
    var normal = vec3<f32>(0.0, 1.0, 0.0);
    if (dot(raw_normal, raw_normal) > 1.0e-12) {
        normal = normalize(raw_normal);
    }
    let to_camera = normalize(post.camera_position.xyz - center_world);
    if (dot(normal, to_camera) < 0.0) { normal = -normal; }
    return normal;
}

fn gather_color(pixel: vec2<i32>, reference_depth: f32) -> vec3<f32> {
    let dims = vec2<f32>(textureDimensions(scene_color));
    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let p = clamp_pixel(pixel + vec2<i32>(x, y));
            let d = depth_at(p);
            let uv = (vec2<f32>(p) + vec2<f32>(0.5)) / dims;
            let offset = vec2<f32>(f32(x), f32(y));
            let spatial = 1.0 / (1.0 + dot(offset, offset));
            let depth_weight = 1.0 - smoothstep(0.002, 0.025, abs(d - reference_depth));
            let weight = spatial * max(depth_weight, 0.05);
            sum += textureSampleLevel(scene_color, scene_sampler, uv, 0.0).rgb * weight;
            weight_sum += weight;
        }
    }
    return sum / max(weight_sum, 1.0e-5);
}

fn edge_confidence(uv: vec2<f32>, fade_width: f32) -> f32 {
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    return smoothstep(0.0, max(fade_width, 0.001), edge);
}

fn depth_continuity(pixel: vec2<i32>, center_depth: f32) -> f32 {
    let d0 = abs(depth_at(pixel + vec2<i32>(1, 0)) - center_depth);
    let d1 = abs(depth_at(pixel + vec2<i32>(-1, 0)) - center_depth);
    let d2 = abs(depth_at(pixel + vec2<i32>(0, 1)) - center_depth);
    let d3 = abs(depth_at(pixel + vec2<i32>(0, -1)) - center_depth);
    let discontinuity = max(max(d0, d1), max(d2, d3));
    return 1.0 - smoothstep(0.006, 0.045, discontinuity);
}

fn previous_uv(world: vec3<f32>) -> vec2<f32> {
    let clip = post.previous_view_proj * vec4<f32>(world, 1.0);
    let safe_w = select(1.0e-6, clip.w, abs(clip.w) > 1.0e-6);
    let ndc = clip.xy / safe_w;
    return ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
}

fn temporal_neighborhood_bounds(uv: vec2<f32>) -> array<vec3<f32>, 2> {
    let dims = vec2<f32>(textureDimensions(scene_color));
    let texel = vec2<f32>(1.0) / max(dims, vec2<f32>(1.0));
    var minimum = vec3<f32>(1.0e6);
    var maximum = vec3<f32>(-1.0e6);
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let sample_uv = clamp(uv + vec2<f32>(f32(x), f32(y)) * texel, vec2<f32>(0.0), vec2<f32>(1.0));
            let color = textureSampleLevel(scene_color, scene_sampler, sample_uv, 0.0).rgb;
            minimum = min(minimum, color);
            maximum = max(maximum, color);
        }
    }
    return array<vec3<f32>, 2>(minimum, maximum);
}

fn apply_temporal_ssr(
    current: vec4<f32>,
    surface_world: vec3<f32>,
    uv: vec2<f32>,
    confidence: f32,
) -> vec4<f32> {
    if (post.p3.x < 0.5 || post.info.z < 0.5) { return current; }
    let history_uv = previous_uv(surface_world);
    if (history_uv.x <= 0.001 || history_uv.x >= 0.999 || history_uv.y <= 0.001 || history_uv.y >= 0.999) {
        return current;
    }
    let history = textureSampleLevel(history_color, scene_sampler, history_uv, 0.0).rgb;
    let bounds = temporal_neighborhood_bounds(uv);
    let expansion = vec3<f32>(max(post.p3.z, 0.0));
    let clamped_history = clamp(history, bounds[0] - expansion, bounds[1] + expansion);
    let threshold = max(post.p3.w, 0.001);
    let difference = length(clamped_history - current.rgb);
    let disocclusion = 1.0 - smoothstep(threshold, threshold * 3.0, difference);
    let reprojection_motion = length(history_uv - uv);
    let motion_confidence = 1.0 - smoothstep(0.015, 0.12, reprojection_motion);
    let weight = clamp(post.p3.y, 0.0, 0.95) * disocclusion * motion_confidence * confidence;
    return vec4<f32>(mix(current.rgb, clamped_history, weight), current.a);
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let base = textureSample(scene_color, scene_sampler, input.uv);
    if (post.p1.w < 0.5) { return base; }

    let dims = vec2<f32>(textureDimensions(scene_depth));
    let pixel = clamp_pixel(vec2<i32>(input.uv * dims));
    let center_depth = depth_at(pixel);
    if (center_depth >= 0.9999) { return base; }

    let surface_world = world_from_depth(pixel, center_depth);
    let normal = reconstructed_normal(pixel, surface_world);
    if (dot(normal, normal) < 0.5) { return base; }
    let view = normalize(post.camera_position.xyz - surface_world);
    let ray = normalize(reflect(-view, normal));
    let origin = surface_world + normal * max(post.p2.x, 0.0);

    let intensity = max(post.p0.x, 0.0);
    let max_distance = max(post.p0.y, 0.1);
    let base_thickness = max(post.p0.z, 0.001);
    let stride = max(post.p0.w, 0.02);
    let max_steps = u32(clamp(post.p1.x, 4.0, 96.0));
    let start_distance = max(post.p1.z, stride * 1.25);

    var previous_delta = 1.0e6;
    var hit_uv = vec2<f32>(-1.0);
    var hit_depth = 1.0;
    var hit_quality = 0.0;
    var hit_travel = max_distance;

    for (var i: u32 = 0u; i < 96u; i = i + 1u) {
        if (i >= max_steps) { break; }
        let travel = min(start_distance + f32(i) * stride, max_distance);
        let ray_world = origin + ray * travel;
        let projected = project_world(ray_world);
        if (projected.x <= 0.001 || projected.x >= 0.999 || projected.y <= 0.001 || projected.y >= 0.999 || projected.z <= 0.0 || projected.z >= 1.0) {
            break;
        }
        let sample_pixel = clamp_pixel(vec2<i32>(projected.xy * dims));
        let sampled_depth = depth_at(sample_pixel);
        if (sampled_depth >= 0.9999) {
            previous_delta = 1.0e6;
            continue;
        }
        let scene_world = world_from_depth(sample_pixel, sampled_depth);
        let ray_camera_distance = length(ray_world - post.camera_position.xyz);
        let scene_camera_distance = length(scene_world - post.camera_position.xyz);
        let delta = scene_camera_distance - ray_camera_distance;
        let thickness = base_thickness * (1.0 + 1.5 * travel / max_distance);
        let crossed = previous_delta > 0.0 && delta <= 0.0;
        let close = abs(delta) <= thickness;
        if ((crossed || close) && travel > start_distance + stride) {
            hit_uv = projected.xy;
            hit_depth = sampled_depth;
            hit_quality = 1.0 - smoothstep(thickness, thickness * 4.5, abs(delta));
            hit_travel = travel;
            break;
        }
        previous_delta = delta;
        if (travel >= max_distance) { break; }
    }

    if (hit_uv.x < 0.0) { return base; }

    let hit_pixel = clamp_pixel(vec2<i32>(hit_uv * dims));
    let hit_world = world_from_depth(hit_pixel, hit_depth);
    let hit_normal = reconstructed_normal(hit_pixel, hit_world);
    let facing = smoothstep(post.p2.z, 1.0, clamp(dot(hit_normal, -ray), 0.0, 1.0));
    let continuity = depth_continuity(hit_pixel, hit_depth);
    let source_edge = edge_confidence(input.uv, post.p1.y * 0.7);
    let hit_edge = edge_confidence(hit_uv, post.p1.y);
    let fade_start = clamp(post.p2.y, 0.0, 1.0) * max_distance;
    let distance_confidence = 1.0 - smoothstep(fade_start, max_distance, hit_travel);
    let fresnel = 0.18 + 0.82 * pow(1.0 - clamp(dot(normal, view), 0.0, 1.0), 5.0);
    let confidence = clamp(
        source_edge * hit_edge * hit_quality * facing * continuity * distance_confidence * fresnel * intensity,
        0.0,
        clamp(post.p2.w, 0.0, 1.0),
    );
    if (confidence <= 0.001) { return base; }

    let reflected = gather_color(hit_pixel, hit_depth);
    let current = vec4<f32>(mix(base.rgb, reflected, confidence), base.a);
    return apply_temporal_ssr(current, surface_world, input.uv, confidence);
}
