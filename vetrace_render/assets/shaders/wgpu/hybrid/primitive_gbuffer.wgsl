// Shared raster G-buffer contract (primitive + mesh passes; consumed by hybrid_compose.comp.wgsl):
// - gbuf_albedo rgba8unorm: rgb = linear base color, a = coverage/valid surface mask.
// - gbuf_normal rgba16float: xyz = world-space normal encoded as normal * 0.5 + 0.5, w = reserved (1.0).
// - gbuf_material rgba8uint: x = metallic UNORM8, y = roughness UNORM8, z = emissive luma UNORM8,
//   w = packed metadata; low nibble = feature flags, high nibble = object/material ID bucket.
// - depth texture r32float: device depth used for world-position reconstruction and sky rejection.
// - gbuf_lightmap_uv rgba16float: xy = authored lightmap UV, z = validity mask, w = object index for editor outline.
const GBUFFER_FEATURE_FLAGS_MASK: u32 = 0x0fu;
const GBUFFER_ID_SHIFT: u32 = 4u;
const GBUFFER_ID_MASK: u32 = 0xf0u;

fn encode_gbuffer_unorm8(v: f32) -> u32 {
    return u32(clamp(v, 0.0, 1.0) * 255.0);
}

fn encode_gbuffer_metadata(id_bucket: u32, feature_flags: u32) -> u32 {
    return ((id_bucket & GBUFFER_FEATURE_FLAGS_MASK) << GBUFFER_ID_SHIFT) | (feature_flags & GBUFFER_FEATURE_FLAGS_MASK);
}

struct Object {
    orientation: vec4<f32>,
    position: vec3<f32>, _pad1: f32,
    size: vec3<f32>, _pad2: f32,
    scale: vec3<f32>, _pad2b: f32,
    material_index: u32,
    radius: f32,
    is_cube: u32,
    is_glass: u32,
    is_mesh: u32,
    triangle_start_idx: u32,
    triangle_count: u32,
    tri_bvh_start: u32,
    tri_bvh_count: u32,
    is_shaded: u32,
    casts_raster_shadow: u32,
    casts_raytraced_shadow: u32,
    shadow_importance: f32,
    max_shadow_distance: f32,
    scene_flags: u32,
    gi_flags: u32,
    _gi_pad0: u32,
    _gi_pad1: u32,
    _struct_pad0: u32,
    _struct_pad1: u32,
};

struct MaterialParams {
    baseColorFactor: vec4<f32>,
    emissiveFactor: vec3<f32>, emissiveStrength: f32,
    metallicFactor: f32,
    roughnessFactor: f32,
    ior: f32,
    baseColorTex: u32,
    f0: vec3<f32>, has_custom_material: u32,
    custom_material_id: u32,
    material_flags0: u32,
    material_flags1: u32,
    material_flags2: u32,
    material_flags3: u32,
    material_flags4: u32,
    material_flags5: u32,
    material_flags6: u32,
};

struct Params {
    camera_pos: vec4<f32>,
    camera_front: vec4<f32>,
    camera_up: vec4<f32>,
    camera_right: vec4<f32>,
    prev_camera_pos: vec4<f32>,
    fov: f32,
    num_objects: i32,
};

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) @interpolate(flat) object_index: u32,
};

@group(0) @binding(0) var<storage, read> objects: array<Object>;
@group(0) @binding(1) var<uniform> view_proj: mat4x4<f32>;
@group(0) @binding(2) var<storage, read> materials: array<MaterialParams>;

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) object_index: u32,
) -> VsOut {
    let obj = objects[object_index];
    let primitive_scale = select(vec3<f32>(obj.radius), obj.size, obj.is_cube != 0u) * obj.scale;
    let local = position * primitive_scale;
    let world = quat_rotate(obj.orientation, local) + obj.position;
    let n = normalize(quat_rotate(obj.orientation, normal));
    var out: VsOut;
    out.pos = view_proj * vec4<f32>(world, 1.0);
    out.world_normal = n;
    out.object_index = object_index;
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
    let obj = objects[in.object_index];
    let mat = materials[obj.material_index];
    var out: FsOut;
    out.albedo = mat.baseColorFactor;
    out.normal = vec4<f32>(normalize(in.world_normal) * 0.5 + vec3<f32>(0.5), 1.0);
    let emissive_luma = max(max(mat.emissiveFactor.r, mat.emissiveFactor.g), mat.emissiveFactor.b) * mat.emissiveStrength;
    let id_bucket = select(obj.material_index, mat.custom_material_id, mat.has_custom_material != 0u);
    let feature_flags = mat.material_flags0;
    out.material = vec4<u32>(
        encode_gbuffer_unorm8(mat.metallicFactor),
        encode_gbuffer_unorm8(mat.roughnessFactor),
        encode_gbuffer_unorm8(emissive_luma),
        encode_gbuffer_metadata(id_bucket, feature_flags),
    );
    out.depth = in.pos.z;
    // Procedural primitives do not carry authored lightmap unwraps; mark invalid.
    // Keep the object index in .w so post-process selection outline survives
    // raster/hybrid composition instead of relying on color alpha as coverage.
    out.lightmap_uv = vec4<f32>(0.0, 0.0, 0.0, f32(in.object_index));
    return out;
}
