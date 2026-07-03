// Bindings start at 1 because binding 0 is used for the view-projection
// matrix in the vertex shader.
@group(0) @binding(1)
var samp: sampler;
@group(0) @binding(2)
var sprite_tex: texture_2d<f32>;

struct FsIn {
    @location(0) frag_uv: vec2<f32>,
};

@fragment
fn fs_main(in: FsIn) -> @location(0) vec4<f32> {
    return textureSample(sprite_tex, samp, in.frag_uv);
}
