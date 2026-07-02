const SIGMA: f32 = 8.0;
const RADIUS: i32 = 8;

@group(0) @binding(0)
var samp: sampler;
@group(0) @binding(1)
var tex: texture_2d<f32>;

struct BlurParams {
    resolution: vec2<f32>,
    region: vec4<f32>,
    feather: f32,
    _pad: vec3<f32>,
};
@group(0) @binding(2)
var<uniform> params: BlurParams;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var positions = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    var out: VsOut;
    out.pos = vec4<f32>(positions[vi], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex_offset = 1.0 / params.resolution;
    let base_uv = in.pos.xy / params.resolution;
    var color = vec4<f32>(0.0);
    var total = 0.0;
    for (var x: i32 = -RADIUS; x <= RADIUS; x = x + 1) {
        for (var y: i32 = -RADIUS; y <= RADIUS; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * tex_offset;
            let weight = exp(-(f32(x * x + y * y)) / (2.0 * SIGMA * SIGMA));
            color += textureSample(tex, samp, base_uv + offset) * weight;
            total += weight;
        }
    }
    let blurred = color / total;
    let original = textureSample(tex, samp, base_uv);
    let frag = in.pos.xy;
    let dx = min(frag.x - params.region.x, params.region.x + params.region.z - frag.x);
    let dy = min(frag.y - params.region.y, params.region.y + params.region.w - frag.y);
    let dist = min(dx, dy);
    let alpha = clamp(dist / params.feather, 0.0, 1.0);
    return mix(original, blurred, alpha);
}