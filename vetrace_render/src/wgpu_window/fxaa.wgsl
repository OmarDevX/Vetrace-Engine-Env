@group(0) @binding(0)
var scene_color: texture_2d<f32>;

@group(0) @binding(1)
var scene_sampler: sampler;

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

fn luma(rgb: vec3<f32>) -> f32 {
    // Green carries most perceived luminance and is cheaper than a full dot product.
    return rgb.g;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(scene_color));
    let texel = 1.0 / max(dimensions, vec2<f32>(1.0));
    let uv = input.uv;

    let center = textureSampleLevel(scene_color, scene_sampler, uv, 0.0);
    let rgb_nw = textureSampleLevel(scene_color, scene_sampler, uv + vec2<f32>(-1.0, -1.0) * texel, 0.0).rgb;
    let rgb_ne = textureSampleLevel(scene_color, scene_sampler, uv + vec2<f32>( 1.0, -1.0) * texel, 0.0).rgb;
    let rgb_sw = textureSampleLevel(scene_color, scene_sampler, uv + vec2<f32>(-1.0,  1.0) * texel, 0.0).rgb;
    let rgb_se = textureSampleLevel(scene_color, scene_sampler, uv + vec2<f32>( 1.0,  1.0) * texel, 0.0).rgb;

    let luma_m = luma(center.rgb);
    let luma_nw = luma(rgb_nw);
    let luma_ne = luma(rgb_ne);
    let luma_sw = luma(rgb_sw);
    let luma_se = luma(rgb_se);
    let luma_min = min(luma_m, min(min(luma_nw, luma_ne), min(luma_sw, luma_se)));
    let luma_max = max(luma_m, max(max(luma_nw, luma_ne), max(luma_sw, luma_se)));
    let luma_range = luma_max - luma_min;

    // Skip flat/low-contrast pixels. This is the main reason the pass stays cheap.
    if (luma_range < max(0.0312, luma_max * 0.125)) {
        return center;
    }

    var direction = vec2<f32>(
        -((luma_nw + luma_ne) - (luma_sw + luma_se)),
         ((luma_nw + luma_sw) - (luma_ne + luma_se))
    );
    let direction_reduce = max((luma_nw + luma_ne + luma_sw + luma_se) * 0.03125, 0.0078125);
    let reciprocal_min = 1.0 / (min(abs(direction.x), abs(direction.y)) + direction_reduce);
    direction = clamp(direction * reciprocal_min, vec2<f32>(-8.0), vec2<f32>(8.0)) * texel;

    let rgb_a = 0.5 * (
        textureSampleLevel(scene_color, scene_sampler, uv + direction * (1.0 / 3.0 - 0.5), 0.0).rgb +
        textureSampleLevel(scene_color, scene_sampler, uv + direction * (2.0 / 3.0 - 0.5), 0.0).rgb
    );
    let rgb_b = rgb_a * 0.5 + 0.25 * (
        textureSampleLevel(scene_color, scene_sampler, uv + direction * -0.5, 0.0).rgb +
        textureSampleLevel(scene_color, scene_sampler, uv + direction *  0.5, 0.0).rgb
    );
    let luma_b = luma(rgb_b);
    let resolved = select(rgb_b, rgb_a, luma_b < luma_min || luma_b > luma_max);
    return vec4<f32>(resolved, center.a);
}
