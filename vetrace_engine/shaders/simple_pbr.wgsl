// Shared raster G-buffer contract (primitive + mesh passes; consumed by hybrid_compose.comp.wgsl):
// - gbuf_albedo rgba8unorm: rgb = linear base color, a = coverage/valid surface mask.
// - gbuf_normal rgba16float: xyz = world-space normal encoded as normal * 0.5 + 0.5, w = reserved (1.0).
// - gbuf_material rgba8uint: x = metallic UNORM8, y = roughness UNORM8, z = emissive luma UNORM8,
//   w = packed metadata; low nibble = feature flags, high nibble = object/material ID bucket.
// - depth texture r32float: device depth used for world-position reconstruction and sky rejection.
// - gbuf_lightmap_uv rgba16float: xy = authored lightmap UV, z = validity mask, w = object index for editor outline.
const GBUFFER_FEATURE_FLAGS_MASK: u32 = 0x0fu;
const GBUFFER_ID_SHIFT: u32 = 4u;

fn encode_gbuffer_unorm8(v: f32) -> u32 {
    return u32(clamp(v, 0.0, 1.0) * 255.0);
}

fn encode_gbuffer_metadata(id_bucket: u32, feature_flags: u32) -> u32 {
    return ((id_bucket & GBUFFER_FEATURE_FLAGS_MASK) << GBUFFER_ID_SHIFT) | (feature_flags & GBUFFER_FEATURE_FLAGS_MASK);
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) object_index: i32,
};

struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    object_index: i32,
    _pad0: vec3<i32>,
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
    out.normal = normalize((uni.model * vec4<f32>(n, 0.0)).xyz);
    out.uv = uv;
    out.object_index = uni.object_index;
    return out;
}

struct FsOut {
    @location(0) albedo: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) material: vec4<u32>,
    @location(3) depth: f32,
    @location(4) lightmap_uv: vec4<f32>,
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
    // Mesh materials currently do not expose emissive, material IDs, or feature flags here;
    // encode explicit zeroes using the shared G-buffer contract so later fields remain stable.
    out.material = vec4<u32>(
        encode_gbuffer_unorm8(m),
        encode_gbuffer_unorm8(r),
        encode_gbuffer_unorm8(0.0),
        encode_gbuffer_metadata(0u, 0u),
    );
    out.depth = in.pos.z;
    out.lightmap_uv = vec4<f32>(in.uv, 1.0, f32(in.object_index));
    return out;
}
