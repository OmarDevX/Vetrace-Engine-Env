use glam::Vec3;
use rapier3d::na as nalgebra;
use rapier3d::na::Point3;
use rapier3d::prelude::{vector, ColliderBuilder, Real};

use crate::components::{Collider, ColliderShape, MeshCollider, MeshColliderShape};

pub(crate) fn collider_builder(collider: &Collider) -> ColliderBuilder {
    let builder = match collider.shape {
        ColliderShape::Sphere => {
            let radii = collider.half_extents.abs().max(Vec3::splat(0.001));
            if nearly_equal(radii.x, radii.y) && nearly_equal(radii.x, radii.z) {
                ColliderBuilder::ball(radii.x)
            } else {
                convex_hull_or_ball(ellipsoid_points(radii, 24, 12), radii.max_element())
            }
        }
        ColliderShape::Cube => ColliderBuilder::cuboid(
            collider.half_extents.x.max(0.001),
            collider.half_extents.y.max(0.001),
            collider.half_extents.z.max(0.001),
        ),
        ColliderShape::Capsule => {
            let extents = collider.half_extents.abs().max(Vec3::splat(0.001));
            let radius = extents.x.max(0.001);
            let total_half_height = extents.y.max(radius + 0.001);
            if nearly_equal(extents.x, extents.z) && total_half_height >= radius {
                let half_segment_height = (total_half_height - radius).max(0.001);
                ColliderBuilder::capsule_y(half_segment_height, radius)
            } else {
                convex_hull_or_ball(elliptical_capsule_points(extents, 24, 8), extents.max_element())
            }
        }
    };
    apply_material(builder, collider.sensor, collider.friction, collider.restitution)
}

pub(crate) fn mesh_collider_builder(collider: &MeshCollider, transform_scale: Vec3) -> Option<ColliderBuilder> {
    let safe_scale = safe_abs_scale(transform_scale);
    let vertices: Vec<Point3<Real>> = collider
        .vertices
        .iter()
        .filter_map(|vertex| {
            let scaled = *vertex * safe_scale;
            scaled.is_finite().then_some(Point3::new(scaled.x as Real, scaled.y as Real, scaled.z as Real))
        })
        .collect();
    if vertices.is_empty() {
        return None;
    }

    let (mut builder, fallback_offset) = match collider.shape {
        MeshColliderShape::ConvexHull => convex_or_bounds_builder(&vertices)?,
        MeshColliderShape::TriangleMesh => {
            let indices = valid_mesh_indices(&collider.indices, vertices.len());
            if indices.is_empty() {
                convex_or_bounds_builder(&vertices)?
            } else {
                match ColliderBuilder::trimesh(vertices.clone(), indices) {
                    Ok(builder) => (builder, Vec3::ZERO),
                    Err(_) => convex_or_bounds_builder(&vertices)?,
                }
            }
        }
    };

    let offset = collider.offset * safe_scale + fallback_offset;
    builder = builder.translation(vector![offset.x, offset.y, offset.z]);
    Some(apply_material(builder, collider.sensor, collider.friction, collider.restitution))
}

fn apply_material(builder: ColliderBuilder, sensor: bool, friction: f32, restitution: f32) -> ColliderBuilder {
    builder
        .sensor(sensor)
        .friction(if friction.is_finite() { friction.max(0.0) } else { 0.7 })
        .restitution(if restitution.is_finite() { restitution.max(0.0) } else { 0.0 })
}

fn convex_or_bounds_builder(vertices: &[Point3<Real>]) -> Option<(ColliderBuilder, Vec3)> {
    if let Some(builder) = ColliderBuilder::convex_hull(vertices) {
        return Some((builder, Vec3::ZERO));
    }
    bounds_cuboid_builder(vertices)
}

fn bounds_cuboid_builder(vertices: &[Point3<Real>]) -> Option<(ColliderBuilder, Vec3)> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for point in vertices {
        let v = Vec3::new(point.x as f32, point.y as f32, point.z as f32);
        min = min.min(v);
        max = max.max(v);
    }
    if !min.is_finite() || !max.is_finite() {
        return None;
    }
    let center = (min + max) * 0.5;
    let half_extents = ((max - min) * 0.5).abs().max(Vec3::splat(0.001));
    Some((ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z), center))
}

fn valid_mesh_indices(indices: &[[u32; 3]], vertex_count: usize) -> Vec<[u32; 3]> {
    indices
        .iter()
        .copied()
        .filter(|tri| tri.iter().all(|index| (*index as usize) < vertex_count))
        .collect()
}

fn nearly_equal(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.001 * a.abs().max(b.abs()).max(1.0)
}

fn convex_hull_or_ball(points: Vec<Point3<Real>>, fallback_radius: f32) -> ColliderBuilder {
    ColliderBuilder::convex_hull(&points).unwrap_or_else(|| ColliderBuilder::ball(fallback_radius.max(0.001)))
}

fn ellipsoid_points(radii: Vec3, sectors: usize, stacks: usize) -> Vec<Point3<Real>> {
    let mut points = Vec::with_capacity((stacks + 1) * sectors);
    for stack in 0..=stacks {
        let v = stack as f32 / stacks.max(1) as f32;
        let phi = std::f32::consts::FRAC_PI_2 - v * std::f32::consts::PI;
        let y = phi.sin();
        let ring = phi.cos();
        for sector in 0..sectors {
            let u = sector as f32 / sectors.max(1) as f32;
            let theta = u * std::f32::consts::TAU;
            points.push(Point3::new(
                (ring * theta.cos() * radii.x) as Real,
                (y * radii.y) as Real,
                (ring * theta.sin() * radii.z) as Real,
            ));
        }
    }
    points
}

fn elliptical_capsule_points(extents: Vec3, sectors: usize, rings_per_cap: usize) -> Vec<Point3<Real>> {
    let cap_y_radius = extents.x.min(extents.z).min(extents.y).max(0.001);
    let cylinder_half = (extents.y - cap_y_radius).max(0.0);
    let mut rings = Vec::<(f32, f32)>::new();

    for i in 0..=rings_per_cap {
        let t = i as f32 / rings_per_cap.max(1) as f32;
        let phi = std::f32::consts::FRAC_PI_2 * (1.0 - t);
        rings.push((cylinder_half + cap_y_radius * phi.sin(), phi.cos()));
    }
    for i in 1..=rings_per_cap {
        let t = i as f32 / rings_per_cap.max(1) as f32;
        let phi = -std::f32::consts::FRAC_PI_2 * t;
        rings.push((-cylinder_half + cap_y_radius * phi.sin(), phi.cos()));
    }

    let mut points = Vec::with_capacity(rings.len() * sectors);
    for (y, ring_factor) in rings {
        for sector in 0..sectors {
            let u = sector as f32 / sectors.max(1) as f32;
            let theta = u * std::f32::consts::TAU;
            points.push(Point3::new(
                (theta.cos() * ring_factor * extents.x) as Real,
                y as Real,
                (theta.sin() * ring_factor * extents.z) as Real,
            ));
        }
    }
    points
}

/// Returns the collider dimensions Rapier should actually use for an entity.
///
/// Rapier rigid bodies carry position/rotation only. They do **not** inherit an
/// ECS `Transform.scale`, so the bridge has to bake the entity scale into the
/// collider shape when creating/updating the Rapier collider. `Collider` stays a
/// local-space authored definition; this helper converts it to physics-space.
pub(crate) fn scaled_collider(collider: &Collider, transform_scale: Vec3) -> Collider {
    let safe_scale = safe_abs_scale(transform_scale);

    let mut scaled = collider.clone();
    scaled.handle = None;
    let scaled_half_extents = (collider.half_extents.abs() * safe_scale).max(Vec3::splat(0.001));
    scaled.half_extents = match collider.shape {
        ColliderShape::Cube => scaled_half_extents,
        // Keep the full axis scale. `collider_builder` uses a normal Rapier ball
        // for uniform scale and a convex-hull ellipsoid for non-uniform scale.
        ColliderShape::Sphere => scaled_half_extents,
        // Keep X/Y/Z scale so non-uniform capsules can be approximated by a
        // convex hull instead of silently becoming a too-wide uniform capsule.
        ColliderShape::Capsule => scaled_half_extents,
    };
    scaled.offset = collider.offset * safe_scale;
    scaled
}

fn safe_abs_scale(transform_scale: Vec3) -> Vec3 {
    Vec3::new(
        finite_or_one(transform_scale.x),
        finite_or_one(transform_scale.y),
        finite_or_one(transform_scale.z),
    )
    .abs()
    .max(Vec3::splat(0.001))
}

fn finite_or_one(value: f32) -> f32 {
    if value.is_finite() { value } else { 1.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ColliderSignature {
    Primitive {
        shape: ColliderShape,
        half_extents: [u32; 3],
        offset: [u32; 3],
        sensor: bool,
        friction: u32,
        restitution: u32,
    },
    Mesh {
        shape: MeshColliderShape,
        vertex_count: u32,
        index_count: u32,
        offset: [u32; 3],
        sensor: bool,
        friction: u32,
        restitution: u32,
        hash: u64,
    },
}

impl ColliderSignature {
    pub(crate) fn from_collider(collider: &Collider) -> Self {
        Self::Primitive {
            shape: collider.shape,
            half_extents: vec3_bits(collider.half_extents),
            offset: vec3_bits(collider.offset),
            sensor: collider.sensor,
            friction: quantized_bits(collider.friction.max(0.0)),
            restitution: quantized_bits(collider.restitution.max(0.0)),
        }
    }

    pub(crate) fn from_mesh_collider(collider: &MeshCollider, scale: Vec3) -> Self {
        let safe_scale = safe_abs_scale(scale);
        Self::Mesh {
            shape: collider.shape,
            vertex_count: collider.vertices.len().min(u32::MAX as usize) as u32,
            index_count: collider.indices.len().min(u32::MAX as usize) as u32,
            offset: vec3_bits(collider.offset * safe_scale),
            sensor: collider.sensor,
            friction: quantized_bits(collider.friction.max(0.0)),
            restitution: quantized_bits(collider.restitution.max(0.0)),
            hash: mesh_hash(collider, safe_scale),
        }
    }
}

fn vec3_bits(value: Vec3) -> [u32; 3] {
    [quantized_bits(value.x), quantized_bits(value.y), quantized_bits(value.z)]
}

fn quantized_bits(value: f32) -> u32 {
    // Avoid rebuilding Rapier shapes for insignificant floating point jitter.
    ((if value.is_finite() { value } else { 0.0 }) * 10_000.0).round().to_bits()
}

fn mesh_hash(collider: &MeshCollider, scale: Vec3) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for vertex in collider.vertices.iter().take(16_384) {
        let scaled = *vertex * scale;
        for bits in vec3_bits(scaled) {
            hash ^= bits as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    for tri in collider.indices.iter().take(16_384) {
        for index in tri {
            hash ^= *index as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

#[derive(Clone, Copy)]
pub(crate) enum BodyKind {
    Dynamic,
    Static,
    Kinematic,
}
