enable wgpu_ray_query;

struct ShadowParams {
    width: u32,
    height: u32,
    frame_index: u32,
    caster_mask: u32,
};

struct RayInstanceMetadata {
    object_id: u32,
    material_table_offset: u32,
    submesh_table_offset: u32,
    flags: u32,
};

@group(0) @binding(0) var scene_tlas: acceleration_structure;
@group(0) @binding(1) var<storage, read> instance_metadata: array<RayInstanceMetadata>;
@group(0) @binding(2) var<uniform> shadow_params: ShadowParams;
@group(0) @binding(3) var shadow_visibility: texture_storage_2d<rgba8unorm, write>;

const SHADOW_CASTER_MASK: u32 = 0x1u;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    if (gid.x >= shadow_params.width || gid.y >= shadow_params.height) {
        return;
    }

    // Integration placeholder: the runtime binds this only when a hardware AS is valid.
    // The final ray origin/direction must match rt_shadows.comp.wgsl so the visibility
    // output can be compared against the software BVH path.
    var query: ray_query<ray_flags::terminate_on_first_hit>;
    let origin = vec3<f32>(0.0, 0.0, 0.0);
    let direction = vec3<f32>(0.0, 1.0, 0.0);
    rayQueryInitialize(&query, scene_tlas, ray_flags::terminate_on_first_hit, SHADOW_CASTER_MASK, origin, 0.001, direction, 100000.0);
    while (rayQueryProceed(&query)) {}
    let visible = select(1.0, 0.0, rayQueryGetIntersectionType(&query, false) != ray_intersection_type::none);
    textureStore(shadow_visibility, vec2<i32>(gid.xy), vec4<f32>(visible, visible, visible, 1.0));
}
