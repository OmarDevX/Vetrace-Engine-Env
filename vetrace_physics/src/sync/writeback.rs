use super::*;

#[derive(Default)]
pub(super) struct PhysicsSyncUpdates {
    pub(super) bodies: Vec<(
        Entity,
        BodyKind,
        rapier3d::prelude::RigidBodyHandle,
    )>,
    pub(super) colliders: Vec<(Entity, rapier3d::prelude::ColliderHandle)>,
    pub(super) mesh_colliders: Vec<(Entity, rapier3d::prelude::ColliderHandle)>,
}

pub(super) fn apply_physics_sync_updates(engine: &mut Engine, updates: PhysicsSyncUpdates) {
    for (entity, kind, handle) in updates.bodies {
        match kind {
            BodyKind::Dynamic => {
                if let Some(body) = engine.raw_world_mut().get_mut::<RigidBody3D>(entity) {
                    body.handle = Some(handle);
                }
            }
            BodyKind::Static => {
                if let Some(body) = engine.raw_world_mut().get_mut::<StaticBody>(entity) {
                    body.handle = Some(handle);
                }
            }
            BodyKind::Kinematic => {
                if let Some(body) = engine.raw_world_mut().get_mut::<KinematicBody>(entity) {
                    body.handle = Some(handle);
                }
            }
        }
    }
    for (entity, handle) in updates.colliders {
        if let Some(collider) = engine.raw_world_mut().get_mut::<Collider>(entity) {
            collider.handle = Some(handle);
        }
    }
    for (entity, handle) in updates.mesh_colliders {
        if let Some(collider) = engine.raw_world_mut().get_mut::<MeshCollider>(entity) {
            collider.handle = Some(handle);
        }
    }
}
