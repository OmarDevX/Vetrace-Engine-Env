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

struct PrimitiveVsOut { @builtin(position) pos: vec4<f32> };
@group(0) @binding(0) var<storage, read> objects: array<Object>;
@group(0) @binding(1) var<uniform> shadow_view_proj: mat4x4<f32>;

fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@vertex
fn primitive_vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) object_index: u32,
) -> PrimitiveVsOut {
    _ = normal;
    let obj = objects[object_index];
    let primitive_scale = select(vec3<f32>(obj.radius), obj.size, obj.is_cube != 0u) * obj.scale;
    let local = position * primitive_scale;
    let world = quat_rotate(obj.orientation, local) + obj.position;
    var out: PrimitiveVsOut;
    out.pos = shadow_view_proj * vec4<f32>(world, 1.0);
    return out;
}

