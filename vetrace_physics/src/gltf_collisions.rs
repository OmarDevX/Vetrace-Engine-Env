use glam::Vec3;
use vetrace_core::{Engine, Transform};
use vetrace_render::{GltfCollisionBodyKind, GltfCollisionShapeKind, GltfImportedCollider};

use crate::components::{
    Collider, ColliderShape, GltfCollisionApplied, KinematicBody, MeshCollider, MeshColliderShape,
    RigidBody3D, StaticBody,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct GltfCollisionApplyReport {
    pub primitive_colliders: usize,
    pub mesh_colliders: usize,
    pub skipped: usize,
}

/// Converts renderer-neutral glTF collision hints into runtime physics
/// components. This is intentionally owned by `vetrace_physics`; `vetrace_render`
/// only reads glTF data and never constructs Rapier state directly.
pub fn apply_gltf_imported_colliders(engine: &mut Engine) -> GltfCollisionApplyReport {
    let imports: Vec<_> = engine.raw_world().query::<GltfImportedCollider>()
        .into_iter()
        .filter(|(entity, _)| !engine.raw_world().has::<GltfCollisionApplied>(*entity))
        .map(|(entity, collider)| (entity, collider.clone()))
        .collect();

    let mut report = GltfCollisionApplyReport::default();
    for (entity, imported) in imports {
        if !engine.raw_world().is_alive(entity) {
            report.skipped += 1;
            continue;
        }
        if !engine.raw_world().has::<Transform>(entity) {
            engine.raw_world_mut().insert(entity, Transform::default());
        }

        let _ = engine.raw_world_mut().remove::<StaticBody>(entity);
        let _ = engine.raw_world_mut().remove::<RigidBody3D>(entity);
        let _ = engine.raw_world_mut().remove::<KinematicBody>(entity);
        let _ = engine.raw_world_mut().remove::<Collider>(entity);
        let _ = engine.raw_world_mut().remove::<MeshCollider>(entity);

        match imported.body {
            GltfCollisionBodyKind::Static => engine.raw_world_mut().insert(entity, StaticBody::default()),
            GltfCollisionBodyKind::Dynamic => engine.raw_world_mut().insert(entity, RigidBody3D::default()),
            GltfCollisionBodyKind::Kinematic => engine.raw_world_mut().insert(entity, KinematicBody::default()),
        }

        match imported.shape {
            GltfCollisionShapeKind::Box => {
                engine.raw_world_mut().insert(entity, primitive_collider(ColliderShape::Cube, &imported));
                report.primitive_colliders += 1;
            }
            GltfCollisionShapeKind::Sphere => {
                engine.raw_world_mut().insert(entity, primitive_collider(ColliderShape::Sphere, &imported));
                report.primitive_colliders += 1;
            }
            GltfCollisionShapeKind::Capsule => {
                engine.raw_world_mut().insert(entity, primitive_collider(ColliderShape::Capsule, &imported));
                report.primitive_colliders += 1;
            }
            GltfCollisionShapeKind::ConvexHull => {
                if imported.vertices.is_empty() {
                    engine.raw_world_mut().insert(entity, primitive_collider(ColliderShape::Cube, &imported));
                    report.primitive_colliders += 1;
                } else {
                    engine.raw_world_mut().insert(entity, mesh_collider(MeshColliderShape::ConvexHull, &imported));
                    report.mesh_colliders += 1;
                }
            }
            GltfCollisionShapeKind::TriangleMesh => {
                if imported.vertices.is_empty() || imported.indices.is_empty() {
                    engine.raw_world_mut().insert(entity, primitive_collider(ColliderShape::Cube, &imported));
                    report.primitive_colliders += 1;
                } else if matches!(imported.body, GltfCollisionBodyKind::Static) {
                    engine.raw_world_mut().insert(entity, mesh_collider(MeshColliderShape::TriangleMesh, &imported));
                    report.mesh_colliders += 1;
                } else {
                    // Moving triangle meshes are expensive and fragile in Rapier.
                    // Keep authored dynamic/kinematic collision safe by falling
                    // back to a convex hull over the same vertices.
                    engine.raw_world_mut().insert(entity, mesh_collider(MeshColliderShape::ConvexHull, &imported));
                    report.mesh_colliders += 1;
                }
            }
        }
        engine.raw_world_mut().insert(entity, GltfCollisionApplied);
    }

    report
}

fn primitive_collider(shape: ColliderShape, imported: &GltfImportedCollider) -> Collider {
    Collider {
        shape,
        half_extents: safe_half_extents(imported.half_extents),
        offset: imported.offset,
        sensor: imported.sensor,
        friction: 0.7,
        restitution: 0.0,
        ..Collider::default()
    }
}

fn mesh_collider(shape: MeshColliderShape, imported: &GltfImportedCollider) -> MeshCollider {
    MeshCollider {
        shape,
        vertices: imported.vertices.clone(),
        indices: imported.indices.clone(),
        offset: imported.offset,
        sensor: imported.sensor,
        friction: 0.7,
        restitution: 0.0,
        ..MeshCollider::default()
    }
}

fn safe_half_extents(value: Vec3) -> Vec3 {
    Vec3::new(
        finite_or(value.x, 0.5),
        finite_or(value.y, 0.5),
        finite_or(value.z, 0.5),
    )
    .abs()
    .max(Vec3::splat(0.001))
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
