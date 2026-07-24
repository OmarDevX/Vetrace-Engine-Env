struct VsOut { @builtin(position) pos: vec4<f32> };
struct PbrUniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    object_index: i32,
    has_skin: u32,
    _pad0: vec2<u32>,
    _pad1: vec4<u32>,
};
@group(0) @binding(0) var<uniform> pbr_uni: PbrUniforms;
@group(0) @binding(1) var<uniform> shadow_view_proj: mat4x4<f32>;
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

@vertex
fn pbr_vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) joints: vec4<u32>,
    @location(5) weights: vec4<f32>
) -> VsOut {
    _ = normal;
    _ = uv;
    var out: VsOut;
    var local = position;
    if (pbr_uni.has_skin != 0u) {
        local = skin_vertex(position, joints, weights);
    }
    let world = pbr_uni.model * vec4<f32>(local, 1.0);
    out.pos = shadow_view_proj * world;
    return out;
}

@vertex
fn pbr_vs_main_static(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(3) uv: vec2<f32>
) -> VsOut {
    _ = normal;
    _ = uv;
    var out: VsOut;
    let world = pbr_uni.model * vec4<f32>(position, 1.0);
    out.pos = shadow_view_proj * world;
    return out;
}
