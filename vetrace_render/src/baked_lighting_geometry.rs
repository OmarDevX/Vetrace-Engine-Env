//! CPU geometry extraction and ray queries used by the baked-lighting tool.

use std::f32::consts::PI;

use glam::{Mat4, Vec2, Vec3};

use crate::backend::RenderObject;
use crate::components::{AlphaMode, PrimitiveShape, Shape};
use crate::resources::{MeshAsset, RenderAssets};

#[derive(Clone, Copy, Debug)]
pub(crate) struct BakeTriangle {
    pub(crate) positions: [Vec3; 3],
    pub(crate) normals: [Vec3; 3],
    pub(crate) lightmap_uvs: Option<[Vec2; 3]>,
    pub(crate) albedo: Vec3,
    pub(crate) emissive: Vec3,
    pub(crate) object_key: u64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RayHit {
    pub(crate) triangle_index: usize,
    pub(crate) position: Vec3,
    pub(crate) normal: Vec3,
}

/// Six isolated UV2 charts in a 3x2 grid. The inset plus post-bake dilation
/// prevents bilinear filtering from bleeding lighting between cube faces.
pub(crate) fn cube_face_lightmap_uv(face_index: usize) -> [Vec2; 4] {
    const CHART_INSET: f32 = 0.10;
    let col = (face_index % 3) as f32;
    let row = (face_index / 3) as f32;
    let cell_min = Vec2::new(col / 3.0, row / 2.0);
    let cell_size = Vec2::new(1.0 / 3.0, 1.0 / 2.0);
    let inset = cell_size * CHART_INSET;
    let min = cell_min + inset;
    let max = cell_min + cell_size - inset;
    [
        Vec2::new(min.x, min.y),
        Vec2::new(min.x, max.y),
        Vec2::new(max.x, max.y),
        Vec2::new(max.x, min.y),
    ]
}

pub(crate) fn append_object_triangles(object: &RenderObject, assets: Option<&RenderAssets>, key: u64, out: &mut Vec<BakeTriangle>) {
    if object.material.alpha_mode == AlphaMode::Blend || object.material.alpha <= 0.001 { return; }
    if let Some(mesh_handle) = object.mesh {
        if let Some(mesh) = assets.and_then(|assets| assets.meshes.get(&mesh_handle.0)) {
            append_mesh_triangles(object, mesh, key, out);
            return;
        }
    }
    let shape = object.shape.clone().unwrap_or(Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE });
    match shape.primitive {
        PrimitiveShape::Cube => append_cube_triangles(object, &shape, key, out),
        PrimitiveShape::Plane | PrimitiveShape::Quad => append_plane_triangles(object, &shape, key, out),
        PrimitiveShape::Sphere => append_sphere_triangles(object, &shape, key, out, 24, 12),
        PrimitiveShape::Capsule => append_capsule_triangles(object, &shape, key, out, 24, 8),
    }
}

fn append_cube_triangles(object: &RenderObject, shape: &Shape, key: u64, out: &mut Vec<BakeTriangle>) {
    let half = shape.size.abs().max(Vec3::splat(0.001)) * 0.5;
    let p = [
        Vec3::new(-half.x, -half.y, -half.z), Vec3::new(half.x, -half.y, -half.z),
        Vec3::new(half.x, half.y, -half.z), Vec3::new(-half.x, half.y, -half.z),
        Vec3::new(-half.x, -half.y, half.z), Vec3::new(half.x, -half.y, half.z),
        Vec3::new(half.x, half.y, half.z), Vec3::new(-half.x, half.y, half.z),
    ];
    let faces = [
        ([0, 3, 2, 1], Vec3::NEG_Z), ([4, 5, 6, 7], Vec3::Z),
        ([0, 4, 7, 3], Vec3::NEG_X), ([1, 2, 6, 5], Vec3::X),
        ([3, 7, 6, 2], Vec3::Y), ([0, 1, 5, 4], Vec3::NEG_Y),
    ];
    for (face_index, (indices, normal)) in faces.into_iter().enumerate() {
        let uv = cube_face_lightmap_uv(face_index);
        append_quad(object, [p[indices[0]], p[indices[1]], p[indices[2]], p[indices[3]]], [normal; 4], uv, key, out);
    }
}

fn append_sphere_triangles(
    object: &RenderObject,
    shape: &Shape,
    key: u64,
    out: &mut Vec<BakeTriangle>,
    sectors: usize,
    stacks: usize,
) {
    let sectors = sectors.max(8);
    let stacks = stacks.max(4);
    let radius = shape.size.max_element().abs().max(0.05) * 0.5;
    let mut vertices = Vec::with_capacity((stacks + 1) * (sectors + 1));
    for stack in 0..=stacks {
        let v = stack as f32 / stacks as f32;
        let phi = std::f32::consts::FRAC_PI_2 - v * PI;
        let y = phi.sin();
        let ring = phi.cos();
        for sector in 0..=sectors {
            let u = sector as f32 / sectors as f32;
            let theta = u * std::f32::consts::TAU;
            let normal = Vec3::new(ring * theta.cos(), y, ring * theta.sin()).normalize_or_zero();
            vertices.push((normal * radius, normal, Vec2::new(u, v)));
        }
    }
    let stride = sectors + 1;
    for stack in 0..stacks {
        for sector in 0..sectors {
            let a = stack * stride + sector;
            let b = (stack + 1) * stride + sector;
            let c = (stack + 1) * stride + sector + 1;
            let d = stack * stride + sector + 1;
            if stack != 0 { append_bake_vertex_triangle(object, &vertices, [a, d, b], key, out); }
            if stack + 1 != stacks { append_bake_vertex_triangle(object, &vertices, [d, c, b], key, out); }
        }
    }
}

fn append_capsule_triangles(
    object: &RenderObject,
    shape: &Shape,
    key: u64,
    out: &mut Vec<BakeTriangle>,
    sectors: usize,
    hemisphere_rings: usize,
) {
    let sectors = sectors.max(8);
    let hemisphere_rings = hemisphere_rings.max(3);
    let radius = (shape.size.x.abs().max(shape.size.z.abs()) * 0.5).max(0.05);
    let total_height = shape.size.y.abs().max(radius * 2.0 + 0.05);
    let cylinder_half = (total_height * 0.5 - radius).max(0.0);
    let mut rings = Vec::new();
    for i in 0..=hemisphere_rings {
        let t = i as f32 / hemisphere_rings as f32;
        let phi = std::f32::consts::FRAC_PI_2 * (1.0 - t);
        rings.push((cylinder_half + radius * phi.sin(), phi.cos()));
    }
    for i in 1..=hemisphere_rings {
        let t = i as f32 / hemisphere_rings as f32;
        let phi = -std::f32::consts::FRAC_PI_2 * t;
        rings.push((-cylinder_half + radius * phi.sin(), phi.cos()));
    }
    let ring_count = rings.len();
    let mut vertices = Vec::with_capacity(ring_count * (sectors + 1));
    for (ring_index, (y, ring_radius_factor)) in rings.into_iter().enumerate() {
        let v = ring_index as f32 / ring_count.saturating_sub(1).max(1) as f32;
        let center_y = if y >= 0.0 { cylinder_half } else { -cylinder_half };
        for sector in 0..=sectors {
            let u = sector as f32 / sectors as f32;
            let theta = u * std::f32::consts::TAU;
            let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
            let xz = radial * (ring_radius_factor * radius);
            let position = Vec3::new(xz.x, y, xz.z);
            let candidate = Vec3::new(xz.x, y - center_y, xz.z).normalize_or_zero();
            let normal = if candidate.length_squared() > 1.0e-8 { candidate } else { Vec3::Y };
            vertices.push((position, normal, Vec2::new(u, v)));
        }
    }
    let stride = sectors + 1;
    for ring in 0..ring_count.saturating_sub(1) {
        for sector in 0..sectors {
            let a = ring * stride + sector;
            let b = (ring + 1) * stride + sector;
            let c = (ring + 1) * stride + sector + 1;
            let d = ring * stride + sector + 1;
            append_bake_vertex_triangle(object, &vertices, [a, d, b], key, out);
            append_bake_vertex_triangle(object, &vertices, [d, c, b], key, out);
        }
    }
}

fn append_bake_vertex_triangle(
    object: &RenderObject,
    vertices: &[(Vec3, Vec3, Vec2)],
    indices: [usize; 3],
    key: u64,
    out: &mut Vec<BakeTriangle>,
) {
    let [a, b, c] = indices.map(|index| vertices[index]);
    append_local_triangle(
        object,
        [a.0, b.0, c.0],
        [a.1, b.1, c.1],
        Some([a.2, b.2, c.2]),
        key,
        out,
    );
}

fn append_plane_triangles(object: &RenderObject, shape: &Shape, key: u64, out: &mut Vec<BakeTriangle>) {
    let half_x = shape.size.x.abs().max(0.001) * 0.5;
    let half_z = shape.size.z.abs().max(shape.size.y.abs()).max(0.001) * 0.5;
    append_quad(
        object,
        [Vec3::new(-half_x, 0.0, -half_z), Vec3::new(-half_x, 0.0, half_z), Vec3::new(half_x, 0.0, half_z), Vec3::new(half_x, 0.0, -half_z)],
        [Vec3::Y; 4],
        [Vec2::new(0.0, 0.0), Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.0)],
        key,
        out,
    );
}

fn append_quad(object: &RenderObject, positions: [Vec3; 4], normals: [Vec3; 4], uvs: [Vec2; 4], key: u64, out: &mut Vec<BakeTriangle>) {
    append_local_triangle(object, [positions[0], positions[1], positions[2]], [normals[0], normals[1], normals[2]], Some([uvs[0], uvs[1], uvs[2]]), key, out);
    append_local_triangle(object, [positions[0], positions[2], positions[3]], [normals[0], normals[2], normals[3]], Some([uvs[0], uvs[2], uvs[3]]), key, out);
}

fn append_mesh_triangles(object: &RenderObject, mesh: &MeshAsset, key: u64, out: &mut Vec<BakeTriangle>) {
    if mesh.vertices.len() < 3 { return; }
    let indices = if mesh.indices.is_empty() { (0..mesh.vertices.len() as u32).collect::<Vec<_>>() } else { mesh.indices.clone() };
    for triangle in indices.chunks_exact(3) {
        let Some(a) = mesh.vertices.get(triangle[0] as usize) else { continue; };
        let Some(b) = mesh.vertices.get(triangle[1] as usize) else { continue; };
        let Some(c) = mesh.vertices.get(triangle[2] as usize) else { continue; };
        let lightmap_uvs = [a.lightmap_uv, b.lightmap_uv, c.lightmap_uv];
        let uv_area = (lightmap_uvs[1] - lightmap_uvs[0]).perp_dot(lightmap_uvs[2] - lightmap_uvs[0]).abs();
        append_local_triangle(
            object,
            [a.position, b.position, c.position],
            [a.normal, b.normal, c.normal],
            (uv_area > 1.0e-8).then_some(lightmap_uvs),
            key,
            out,
        );
    }
}

fn append_local_triangle(
    object: &RenderObject,
    positions: [Vec3; 3],
    normals: [Vec3; 3],
    lightmap_uvs: Option<[Vec2; 3]>,
    key: u64,
    out: &mut Vec<BakeTriangle>,
) {
    let model = Mat4::from_scale_rotation_translation(object.transform.scale, object.transform.rotation, object.transform.translation);
    let normal_matrix = if model.determinant().abs() > 1.0e-8 { model.inverse().transpose() } else { Mat4::IDENTITY };
    let positions = positions.map(|position| model.transform_point3(position));
    if (positions[1] - positions[0]).cross(positions[2] - positions[0]).length_squared() <= 1.0e-10 { return; }
    let normals = normals.map(|normal| normal_matrix.transform_vector3(normal).normalize_or_zero());
    out.push(BakeTriangle {
        positions,
        normals,
        lightmap_uvs,
        albedo: object.material.base_color.clamp(Vec3::ZERO, Vec3::ONE),
        emissive: object.material.emissive.max(Vec3::ZERO),
        object_key: key,
    });
}

pub(crate) fn trace_any(origin: Vec3, direction: Vec3, max_distance: f32, ignore: Option<usize>, triangles: &[BakeTriangle]) -> bool {
    trace_nearest(origin, direction, max_distance, ignore, triangles).is_some()
}

pub(crate) fn trace_nearest(origin: Vec3, direction: Vec3, max_distance: f32, ignore: Option<usize>, triangles: &[BakeTriangle]) -> Option<RayHit> {
    let direction = direction.normalize_or_zero();
    if direction.length_squared() <= 1.0e-8 { return None; }
    let mut nearest = max_distance;
    let mut result = None;
    for (index, triangle) in triangles.iter().enumerate() {
        if Some(index) == ignore { continue; }
        let Some((distance, u, v)) = intersect_triangle(origin, direction, triangle.positions) else { continue; };
        if distance <= 0.00001 || distance >= nearest { continue; }
        let w = 1.0 - u - v;
        let normal = (triangle.normals[0] * w + triangle.normals[1] * u + triangle.normals[2] * v).normalize_or_zero();
        nearest = distance;
        result = Some(RayHit { triangle_index: index, position: origin + direction * distance, normal });
    }
    result
}

fn intersect_triangle(origin: Vec3, direction: Vec3, p: [Vec3; 3]) -> Option<(f32, f32, f32)> {
    let edge1 = p[1] - p[0];
    let edge2 = p[2] - p[0];
    let h = direction.cross(edge2);
    let a = edge1.dot(h);
    if a.abs() < 1.0e-7 { return None; }
    let f = 1.0 / a;
    let s = origin - p[0];
    let u = f * s.dot(h);
    if !(0.0..=1.0).contains(&u) { return None; }
    let q = s.cross(edge1);
    let v = f * direction.dot(q);
    if v < 0.0 || u + v > 1.0 { return None; }
    let t = f * edge2.dot(q);
    (t > 1.0e-6).then_some((t, u, v))
}

pub(crate) fn barycentric_2d(point: Vec2, triangle: [Vec2; 3]) -> Option<Vec3> {
    let v0 = triangle[1] - triangle[0];
    let v1 = triangle[2] - triangle[0];
    let v2 = point - triangle[0];
    let denominator = v0.perp_dot(v1);
    if denominator.abs() <= 1.0e-8 { return None; }
    let y = v2.perp_dot(v1) / denominator;
    let z = v0.perp_dot(v2) / denominator;
    Some(Vec3::new(1.0 - y - z, y, z))
}

pub(crate) fn triangle_bounds(triangles: &[BakeTriangle]) -> Option<(Vec3, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for triangle in triangles {
        for position in triangle.positions { min = min.min(position); max = max.max(position); }
    }
    (min.is_finite() && max.is_finite() && max.cmpgt(min).any()).then_some((min, max))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_lightmap_charts_stay_inside_uv_space() {
        for face in 0..6 {
            for uv in cube_face_lightmap_uv(face) {
                assert!((0.0..=1.0).contains(&uv.x));
                assert!((0.0..=1.0).contains(&uv.y));
            }
        }
    }

    #[test]
    fn cube_lightmap_faces_have_separate_chart_centers() {
        let mut centers = Vec::new();
        for face in 0..6 {
            let chart = cube_face_lightmap_uv(face);
            centers.push((chart[0] + chart[1] + chart[2] + chart[3]) * 0.25);
        }
        for a in 0..centers.len() {
            for b in a + 1..centers.len() {
                assert!(centers[a].distance_squared(centers[b]) > 0.01);
            }
        }
    }
}
