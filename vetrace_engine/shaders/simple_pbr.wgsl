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

@group(0) @binding(0) var<uniform> uni: Uniforms;
@group(0) @binding(1) var base_color_tex: texture_2d<f32>;
@group(0) @binding(2) var base_color_sampler: sampler;
@group(0) @binding(3) var<uniform> mat: MaterialUniforms;
@group(0) @binding(4) var<uniform> joint_mats: array<mat4x4<f32>, 64>;

fn skin_vertex(pos: vec3<f32>, joints: vec4<u32>, weights: vec4<f32>) -> vec3<f32> {
    var p = vec4<f32>(pos, 1.0);
    var skinned = vec4<f32>(0.0);
    for (var i: i32 = 0; i < 4; i = i + 1) {
        let j = joints[i];
        skinned = skinned + (joint_mats[j] * p) * weights[i];
    }
    return skinned.xyz;
}

fn skin_normal(n: vec3<f32>, joints: vec4<u32>, weights: vec4<f32>) -> vec3<f32> {
    var p = vec4<f32>(n, 0.0);
    var skinned = vec4<f32>(0.0);
    for (var i: i32 = 0; i < 4; i = i + 1) {
        let j = joints[i];
        skinned = skinned + (joint_mats[j] * p) * weights[i];
    }
    return skinned.xyz;
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>
) -> VsOut {
    var out: VsOut;
    let p = skin_vertex(position, joints, weights);
    let n = skin_normal(normal, joints, weights);
    out.pos = uni.mvp * vec4<f32>(p, 1.0);
    out.normal = n;
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
    out.albedo = base;
    out.normal = vec4<f32>(normalize(in.normal) * 0.5 + vec3<f32>(0.5), 1.0);
    let m = clamp(mat.metallic, 0.0, 1.0);
    let r = clamp(mat.roughness, 0.0, 1.0);
    out.material = vec4<u32>(u32(m * 255.0), u32(r * 255.0), 0u, 0u);
    return out;
}