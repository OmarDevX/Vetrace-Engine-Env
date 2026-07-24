struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let world = object.model * vec4<f32>(input.position, 1.0);
    out.world_position = world.xyz;
    out.normal = normalize((object.normal_model * vec4<f32>(input.normal, 0.0)).xyz);
    out.uv = input.uv;
    out.position = camera.view_proj * world;
    return out;
}
