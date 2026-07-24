use super::*;

pub(super) fn sync_dynamic_bodies(
    state: &mut PhysicsState,
    bodies: Vec<DynamicBodyEntry>,
    updates: &mut PhysicsSyncUpdates,
) {
    for (entity, body, transform) in bodies {
        let pose = transform.pose();
        let handle = state
            .entity_bodies
            .get(&entity)
            .copied()
            .or(body.handle)
            .unwrap_or_else(|| {
                let rigid_body = RigidBodyBuilder::dynamic()
                    .position(isometry_from_pose(pose))
                    .build();
                let handle = state.bodies.insert(rigid_body);
                state.entity_bodies.insert(entity, handle);
                state.transform_cache.insert(entity, pose);
                handle
            });
        let externally_changed =
            transform_changed_externally(state.transform_cache.get(&entity).copied(), pose);
        if let Some(rigid_body) = state.bodies.get_mut(handle) {
            if !rigid_body.is_dynamic() {
                rigid_body.set_body_type(RigidBodyType::Dynamic, true);
            }
            if externally_changed {
                rigid_body.set_position(isometry_from_pose(pose), true);
            }
        }
        if externally_changed {
            state.transform_cache.insert(entity, pose);
        }
        updates.bodies.push((entity, BodyKind::Dynamic, handle));
    }
}

pub(super) fn sync_static_bodies(
    state: &mut PhysicsState,
    bodies: Vec<StaticBodyEntry>,
    updates: &mut PhysicsSyncUpdates,
) {
    for (entity, body, transform) in bodies {
        let pose = transform.pose();
        let handle = state
            .entity_bodies
            .get(&entity)
            .copied()
            .or(body.handle)
            .unwrap_or_else(|| {
                let rigid_body = RigidBodyBuilder::fixed()
                    .position(isometry_from_pose(pose))
                    .build();
                let handle = state.bodies.insert(rigid_body);
                state.entity_bodies.insert(entity, handle);
                handle
            });
        if let Some(rigid_body) = state.bodies.get_mut(handle) {
            if !rigid_body.is_fixed() {
                rigid_body.set_body_type(RigidBodyType::Fixed, true);
            }
            rigid_body.set_position(isometry_from_pose(pose), true);
            state.transform_cache.insert(entity, pose);
        }
        updates.bodies.push((entity, BodyKind::Static, handle));
    }
}

pub(super) fn sync_kinematic_bodies(
    state: &mut PhysicsState,
    bodies: Vec<KinematicBodyEntry>,
    updates: &mut PhysicsSyncUpdates,
) {
    for (entity, body, transform) in bodies {
        let pose = transform.pose();
        let handle = state
            .entity_bodies
            .get(&entity)
            .copied()
            .or(body.handle)
            .unwrap_or_else(|| {
                let rigid_body = RigidBodyBuilder::kinematic_position_based()
                    .position(isometry_from_pose(pose))
                    .build();
                let handle = state.bodies.insert(rigid_body);
                state.entity_bodies.insert(entity, handle);
                handle
            });
        if let Some(rigid_body) = state.bodies.get_mut(handle) {
            if !rigid_body.is_kinematic() {
                rigid_body.set_body_type(RigidBodyType::KinematicPositionBased, true);
            }
            rigid_body.set_next_kinematic_position(isometry_from_pose(pose));
            rigid_body.set_position(isometry_from_pose(pose), true);
            state.transform_cache.insert(entity, pose);
        }
        updates.bodies.push((entity, BodyKind::Kinematic, handle));
    }
}

pub(super) fn sync_body_velocities(
    state: &mut PhysicsState,
    linear: Vec<(Entity, Vec3)>,
    angular: Vec<(Entity, Vec3)>,
) {
    for (entity, velocity) in linear {
        let Some(handle) = state.entity_bodies.get(&entity).copied() else {
            continue;
        };
        if let Some(body) = state.bodies.get_mut(handle) {
            body.set_linvel(
                rapier3d::na::Vector3::new(velocity.x, velocity.y, velocity.z),
                true,
            );
        }
    }
    for (entity, velocity) in angular {
        let Some(handle) = state.entity_bodies.get(&entity).copied() else {
            continue;
        };
        if let Some(body) = state.bodies.get_mut(handle) {
            body.set_angvel(
                rapier3d::na::Vector3::new(velocity.x, velocity.y, velocity.z),
                true,
            );
        }
    }
}
