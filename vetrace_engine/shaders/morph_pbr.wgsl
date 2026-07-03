struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
};

struct MaterialUniforms {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
    _pad: vec2<f32>,
};

struct MorphWeights {
    weights: array<f32, 8>, // Support up to 8 morph targets
};

@group(0) @binding(0) var<uniform> uni: Uniforms;
@group(0) @binding(1) var base_color_tex: texture_2d<f32>;
@group(0) @binding(2) var base_color_sampler: sampler;
@group(0) @binding(3) var<uniform> mat: MaterialUniforms;
@group(0) @binding(4) var<uniform> morph_weights: MorphWeights;
@group(0) @binding(5) var<uniform> joint_mats: array<mat4x4<f32>, 64>;

// Morph target data - positions and normals for each target
@group(1) @binding(0) var<storage, read> morph_positions_0: array<vec3<f32>>;
@group(1) @binding(1) var<storage, read> morph_positions_1: array<vec3<f32>>;
@group(1) @binding(2) var<storage, read> morph_positions_2: array<vec3<f32>>;
@group(1) @binding(3) var<storage, read> morph_positions_3: array<vec3<f32>>;
@group(1) @binding(4) var<storage, read> morph_normals_0: array<vec3<f32>>;
@group(1) @binding(5) var<storage, read> morph_normals_1: array<vec3<f32>>;
@group(1) @binding(6) var<storage, read> morph_normals_2: array<vec3<f32>>;
@group(1) @binding(7) var<storage, read> morph_normals_3: array<vec3<f32>>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>, 
    @location(1) normal: vec3<f32>, 
    @location(3) uv: vec2<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>,
    @builtin(vertex_index) vertex_index: u32
) -> VsOut {
    var out: VsOut;

    // Start with base position and normal
    var morphed_position = position;
    var morphed_normal = normal;
    
    // Apply morph target blending
    // Note: This is a simplified version - in practice you'd want to handle
    // variable numbers of morph targets more efficiently
    
    if (morph_weights.weights[0] != 0.0) {
        morphed_position += morph_positions_0[vertex_index] * morph_weights.weights[0];
        morphed_normal += morph_normals_0[vertex_index] * morph_weights.weights[0];
    }
    
    if (morph_weights.weights[1] != 0.0) {
        morphed_position += morph_positions_1[vertex_index] * morph_weights.weights[1];
        morphed_normal += morph_normals_1[vertex_index] * morph_weights.weights[1];
    }
    
    if (morph_weights.weights[2] != 0.0) {
        morphed_position += morph_positions_2[vertex_index] * morph_weights.weights[2];
        morphed_normal += morph_normals_2[vertex_index] * morph_weights.weights[2];
    }
    
    if (morph_weights.weights[3] != 0.0) {
        morphed_position += morph_positions_3[vertex_index] * morph_weights.weights[3];
        morphed_normal += morph_normals_3[vertex_index] * morph_weights.weights[3];
    }
    
    // Apply skinning
    var p = vec4<f32>(morphed_position, 1.0);
    var n = vec4<f32>(morphed_normal, 0.0);
    var skinned_p = vec4<f32>(0.0);
    var skinned_n = vec4<f32>(0.0);
    for (var i: i32 = 0; i < 4; i = i + 1) {
        let j = joints[i];
        skinned_p = skinned_p + (joint_mats[j] * p) * weights[i];
        skinned_n = skinned_n + (joint_mats[j] * n) * weights[i];
    }
    out.pos = uni.mvp * skinned_p;
    out.normal = normalize(skinned_n.xyz);
    out.uv = uv;
    return out;
}

struct FsOut {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) material: vec4<u32>,
};

@fragment
fn fs_main(in: VsOut) -> FsOut {
    var out: FsOut;
    let base = textureSample(base_color_tex, base_color_sampler, in.uv) * mat.base_color;

    // G-buffer alpha is a coverage/valid-surface mask consumed by SSR/compose.
    // Do not write OBJ/MTL opacity here: some opaque assets, including Sponza,
    // ship with `d 0.0` in the MTL, which would make the compose pass replace
    // the mesh with sky/background. This mesh pass is opaque, so mark covered
    // fragments as valid. Real transparent materials should use a separate pass.
    out.albedo = vec4<f32>(base.rgb, 1.0);
    out.normal = vec4<f32>(normalize(in.normal) * 0.5 + vec3<f32>(0.5), 1.0);
    let m = clamp(mat.metallic, 0.0, 1.0);
    let r = clamp(mat.roughness, 0.0, 1.0);
    out.material = vec4<u32>(u32(m * 255.0), u32(r * 255.0), 0u, 0u);
    return out;
}
