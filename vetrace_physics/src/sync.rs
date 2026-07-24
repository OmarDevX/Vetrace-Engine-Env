use std::collections::HashSet;

use glam::Vec3;
use rapier3d::prelude::{RigidBodyBuilder, RigidBodyType};
use vetrace_core::components::builtins::{GlobalTransform, Parent, Transform};
use vetrace_core::ecs::Entity;
use vetrace_core::engine::Engine;

use crate::cleanup::{remove_missing_physics_entities, remove_unowned_physics_entities};
use crate::colliders::{
    collider_builder, mesh_collider_builder, scaled_collider, BodyKind, ColliderSignature,
};
use crate::components::{
    AngularVelocity, Collider, KinematicBody, MeshCollider, RigidBody3D, StaticBody, Velocity,
};
use crate::state::{
    isometry_from_pose, pose_from_body, transform_changed_externally, PhysicsPose, PhysicsState,
};

mod bodies;
mod collect;
mod colliders;
mod transforms;
mod writeback;

use bodies::*;
use collect::*;
use colliders::*;
use transforms::*;
use writeback::*;

pub(crate) fn sync_world_to_rapier(engine: &mut Engine) {
    let snapshot = collect_physics_sync_snapshot(engine);
    let mut updates = PhysicsSyncUpdates::default();

    if let Some(state) = engine.get_resource_mut::<PhysicsState>() {
        remove_missing_physics_entities(state, &snapshot.live_entities);
        remove_unowned_physics_entities(
            state,
            &snapshot.body_owner_entities,
            &snapshot.collider_owner_entities,
        );
        sync_dynamic_bodies(state, snapshot.dynamic_bodies, &mut updates);
        sync_static_bodies(state, snapshot.static_bodies, &mut updates);
        sync_kinematic_bodies(state, snapshot.kinematic_bodies, &mut updates);
        sync_body_velocities(
            state,
            snapshot.velocities,
            snapshot.angular_velocities,
        );
        sync_colliders(
            state,
            snapshot.colliders,
            snapshot.mesh_colliders,
            &mut updates,
        );
        state.query_pipeline.update(&state.colliders);
    }

    apply_physics_sync_updates(engine, updates);
}

pub(crate) fn sync_rapier_to_world(engine: &mut Engine) {
    let body_states: Vec<_> = engine
        .get_resource::<PhysicsState>()
        .map(|state| {
            state
                .entity_bodies
                .iter()
                .filter_map(|(entity, handle)| {
                    let body = state.bodies.get(*handle)?;
                    if !body.is_dynamic() {
                        return None;
                    }
                    let pose = pose_from_body(body);
                    let linvel = body.linvel();
                    let angvel = body.angvel();
                    Some((
                        *entity,
                        pose,
                        Vec3::new(linvel.x, linvel.y, linvel.z),
                        Vec3::new(angvel.x, angvel.y, angvel.z),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    for (entity, pose, linear, angular) in &body_states {
        write_world_pose_to_local_transform(engine, *entity, *pose);
        if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity>(*entity) {
            velocity.linear = *linear;
        }
        if let Some(velocity) = engine
            .raw_world_mut()
            .get_mut::<AngularVelocity>(*entity)
        {
            velocity.angular = *angular;
        }
    }

    if let Some(state) = engine.get_resource_mut::<PhysicsState>() {
        for (entity, pose, _, _) in body_states {
            state.transform_cache.insert(entity, pose);
        }
    }
}
