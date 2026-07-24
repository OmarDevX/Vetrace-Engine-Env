use super::*;

pub(super) fn sync_colliders(
    state: &mut PhysicsState,
    colliders: Vec<ColliderEntry>,
    mesh_colliders: Vec<MeshColliderEntry>,
    updates: &mut PhysicsSyncUpdates,
) {
    for (entity, collider, transform) in colliders {
        let Some(body_handle) = state.entity_bodies.get(&entity).copied() else {
            continue;
        };
        let effective_collider = scaled_collider(&collider, transform.scale());
        let signature = ColliderSignature::from_collider(&effective_collider);
        let builder = collider_builder(&effective_collider).translation(
            rapier3d::na::Vector3::new(
                effective_collider.offset.x,
                effective_collider.offset.y,
                effective_collider.offset.z,
            ),
        );
        let handle = sync_collider_handle(state, entity, body_handle, signature, builder);
        updates.colliders.push((entity, handle));
    }

    for (entity, collider, transform) in mesh_colliders {
        let Some(body_handle) = state.entity_bodies.get(&entity).copied() else {
            continue;
        };
        let signature = ColliderSignature::from_mesh_collider(&collider, transform.scale());
        let Some(builder) = mesh_collider_builder(&collider, transform.scale()) else {
            continue;
        };
        let handle = sync_collider_handle(state, entity, body_handle, signature, builder);
        updates.mesh_colliders.push((entity, handle));
    }
}

fn sync_collider_handle(
    state: &mut PhysicsState,
    entity: Entity,
    body_handle: rapier3d::prelude::RigidBodyHandle,
    signature: ColliderSignature,
    builder: rapier3d::prelude::ColliderBuilder,
) -> rapier3d::prelude::ColliderHandle {
    let cached_signature = state.collider_cache.get(&entity).copied();
    let cached_handle = state.entity_colliders.get(&entity).copied();

    if cached_signature == Some(signature) {
        return cached_handle.unwrap_or_else(|| {
            let handle = state.colliders.insert_with_parent(
                builder.build(),
                body_handle,
                &mut state.bodies,
            );
            state.entity_colliders.insert(entity, handle);
            state.collider_entities.insert(handle, entity);
            state.collider_cache.insert(entity, signature);
            handle
        });
    }

    if let Some(old_handle) = cached_handle {
        state.entity_colliders.remove(&entity);
        state.collider_entities.remove(&old_handle);
        let _ = state
            .colliders
            .remove(old_handle, &mut state.islands, &mut state.bodies, true);
    }
    let handle =
        state
            .colliders
            .insert_with_parent(builder.build(), body_handle, &mut state.bodies);
    state.entity_colliders.insert(entity, handle);
    state.collider_entities.insert(handle, entity);
    state.collider_cache.insert(entity, signature);
    handle
}
