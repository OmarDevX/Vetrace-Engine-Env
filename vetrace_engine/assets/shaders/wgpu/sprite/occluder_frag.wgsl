@group(0) @binding(1)
var samp: sampler;
@group(0) @binding(2)
var sprite_tex: texture_2d<f32>;

struct FsIn {
    @location(0) frag_uv: vec2<f32>,
};

@fragment
fn fs_main(in: FsIn) -> @location(0) f32 {
    // write sprite alpha directly so the occluder mask retains smooth edges
    let alpha = textureSample(sprite_tex, samp, in.frag_uv).a;
    return alpha;
}