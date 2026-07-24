// Simple Shooter game-side outline shell shader.
// Drawn on a slightly-expanded duplicate player shape with front-face culling,
// no depth writes, LessEqual depth testing, and an overlay bucket selected by
// CustomShaderMaterial. Keep this shader opaque: the shell/depth test already
// creates the silhouette, and rim-faded alpha makes the outline look washed out.

struct VetraceCustomParams {
    params: array<vec4<f32>, 4>,
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    time_health: vec4<f32>,
};

struct MaterialFragmentInput {
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> vetrace_custom: VetraceCustomParams;

@fragment
fn fs_main(_input: MaterialFragmentInput) -> @location(0) vec4<f32> {
    let color = clamp(vetrace_custom.color_a.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let alpha = clamp(vetrace_custom.params[0].w, 0.0, 1.0);
    return vec4<f32>(color, alpha);
}
