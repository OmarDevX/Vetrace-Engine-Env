// EXPERIMENTAL/FUTURE: moved out of the active hybrid shader directory because Rust does not wire this shader into a pipeline yet. See docs/SHADER_ARCHITECTURE.md.
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn main(@location(0) position: vec3<f32>, @location(1) uv: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(position, 1.0);
    out.uv = uv;
    return out;
}
