struct Camera2DUniform {
    center_rotation: vec4<f32>,
    surface_scale_snap: vec4<f32>,
    viewport_rect: vec4<f32>,
};

struct VertexInput {
    @location(0) axis_x_origin: vec4<f32>,
    @location(1) axis_y_uv_origin: vec4<f32>,
    @location(2) uv_delta_cutoff_snap: vec4<f32>,
    @location(3) tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) alpha_cutoff: f32,
};

@group(0) @binding(0)
var<uniform> camera: Camera2DUniform;
@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

fn quad_position(vertex_index: u32) -> vec2<f32> {
    // Naga 0.20 represents a function-local `let` array as a composed
    // expression, which may only be indexed with a compile-time constant.
    // Keep the six-vertex triangle-list lookup branch-based so the shader is
    // valid on the native Vulkan path and WebGPU.
    if (vertex_index == 0u || vertex_index == 3u) {
        return vec2<f32>(0.0, 0.0);
    }
    if (vertex_index == 1u) {
        return vec2<f32>(1.0, 0.0);
    }
    if (vertex_index == 2u || vertex_index == 4u) {
        return vec2<f32>(1.0, 1.0);
    }
    return vec2<f32>(0.0, 1.0);
}

@vertex
fn vs_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let q = quad_position(vertex_index);
    let axis_x = input.axis_x_origin.xy;
    let origin = input.axis_x_origin.zw;
    let axis_y = input.axis_y_uv_origin.xy;
    var world = origin + axis_x * q.x + axis_y * q.y;

    let relative = world - camera.center_rotation.xy;
    let c = camera.center_rotation.z;
    let s = camera.center_rotation.w;
    var camera_local = vec2<f32>(
        c * relative.x + s * relative.y,
        -s * relative.x + c * relative.y,
    );
    var pixel = camera_local * camera.surface_scale_snap.z;
    if (camera.surface_scale_snap.w > 0.5 || input.uv_delta_cutoff_snap.w > 0.5) {
        pixel = round(pixel);
    }

    var output: VertexOutput;
    let screen = camera.viewport_rect.xy
        + camera.viewport_rect.zw * 0.5
        + vec2<f32>(pixel.x, -pixel.y);
    output.position = vec4<f32>(
        screen.x * 2.0 / max(camera.surface_scale_snap.x, 1.0) - 1.0,
        1.0 - screen.y * 2.0 / max(camera.surface_scale_snap.y, 1.0),
        0.0,
        1.0,
    );
    output.uv = input.axis_y_uv_origin.zw + q * input.uv_delta_cutoff_snap.xy;
    output.tint = input.tint;
    output.alpha_cutoff = input.uv_delta_cutoff_snap.z;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(sprite_texture, sprite_sampler, input.uv) * input.tint;
    if (color.a < input.alpha_cutoff) {
        discard;
    }
    color = vec4<f32>(color.rgb * color.a, color.a);
    return color;
}
