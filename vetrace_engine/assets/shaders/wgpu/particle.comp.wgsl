struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    lifetime: f32,
    _pad: vec3<f32>,
};

struct Params {
    dt: f32,
    count: u32,
    _pad: vec2<u32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.count) { return; }
    var p: Particle = particles[id.x];
    let next_position = p.position.xyz + p.velocity.xyz * params.dt;
    p.position = vec4<f32>(next_position, p.position.w);
    p.lifetime = p.lifetime - params.dt;
    particles[id.x] = p;
}
