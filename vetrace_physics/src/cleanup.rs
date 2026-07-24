use std::collections::HashSet;

use glam::Vec3;
use rapier3d::na as nalgebra;
use rapier3d::prelude::{vector, ColliderHandle};
use vetrace_core::components::builtins::Transform;
use vetrace_core::ecs::Entity;
use vetrace_core::engine::Engine;

use crate::components::{AngularVelocity, Velocity};
use crate::state::{isometry_from_pose, PhysicsPose, PhysicsState};

/// Teleports a Rapier-backed entity immediately.
///
/// Most code should only change `Transform`/`Velocity`; the physics bridge now
/// detects external Transform edits and synchronizes Rapier automatically before
/// stepping. This helper remains for same-frame teleports where callers want the
/// query pipeline updated immediately.
pub fn teleport_body(engine: &mut Engine, entity: Entity, position: Vec3, linear_velocity: Vec3) {
    let mut pose = None;
    if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
        transform.translation = position;
        pose = Some(PhysicsPose::from_transform(transform));
    }
    if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity>(entity) {
        velocity.linear = linear_velocity;
    }
    if let Some(angular_velocity) = engine.raw_world_mut().get_mut::<AngularVelocity>(entity) {
        angular_velocity.angular = Vec3::ZERO;
    }

    let mut touched_body = false;
    if let Some(physics) = engine.get_resource_mut::<PhysicsState>() {
        if let Some(handle) = physics.entity_bodies.get(&entity).copied() {
            if let Some(body) = physics.bodies.get_mut(handle) {
                let pose = pose.unwrap_or(PhysicsPose { translation: position, rotation: glam::Quat::IDENTITY });
                body.set_position(isometry_from_pose(pose), true);
                body.set_linvel(vector![linear_velocity.x, linear_velocity.y, linear_velocity.z], true);
                body.set_angvel(vector![0.0, 0.0, 0.0], true);
                physics.transform_cache.insert(entity, pose);
                touched_body = true;
            }
        }
        if touched_body {
            physics.query_pipeline.update(&physics.colliders);
        }
    }
}

/// Removes all Rapier state owned by an ECS entity.
///
/// `World::despawn` only removes ECS components. Rapier keeps its own body and
/// collider sets, so editor/game deletes must also call this helper; otherwise
/// invisible old colliders keep affecting physics and raycasts.
pub fn remove_physics_entity(engine: &mut Engine, entity: Entity) {
    let mut removed = false;
    if let Some(physics) = engine.get_resource_mut::<PhysicsState>() {
        if let Some(collider_handle) = physics.entity_colliders.remove(&entity) {
            physics.collider_entities.remove(&collider_handle);
        }

        physics.transform_cache.remove(&entity);
        physics.collider_cache.remove(&entity);

        if let Some(body_handle) = physics.entity_bodies.remove(&entity) {
            physics.bodies.remove(
                body_handle,
                &mut physics.islands,
                &mut physics.colliders,
                &mut physics.impulse_joints,
                &mut physics.multibody_joints,
                true,
            );
            removed = true;
        }

        if removed {
            // `RigidBodySet::remove(..., true)` deletes attached colliders, so
            // scrub stale reverse-map entries too.
            scrub_stale_collider_maps(physics, None);
            physics.query_pipeline.update(&physics.colliders);
        }
    }
}

pub(crate) fn remove_missing_physics_entities(state: &mut PhysicsState, live_entities: &HashSet<Entity>) {
    let stale: Vec<Entity> = state
        .entity_bodies
        .keys()
        .copied()
        .filter(|entity| !live_entities.contains(entity))
        .collect();

    for entity in stale {
        if let Some(collider_handle) = state.entity_colliders.remove(&entity) {
            state.collider_entities.remove(&collider_handle);
        }
        state.transform_cache.remove(&entity);
        state.collider_cache.remove(&entity);
        if let Some(body_handle) = state.entity_bodies.remove(&entity) {
            state.bodies.remove(
                body_handle,
                &mut state.islands,
                &mut state.colliders,
                &mut state.impulse_joints,
                &mut state.multibody_joints,
                true,
            );
        }
    }

    scrub_stale_collider_maps(state, Some(live_entities));
}

pub(crate) fn remove_unowned_physics_entities(
    state: &mut PhysicsState,
    body_owner_entities: &HashSet<Entity>,
    collider_owner_entities: &HashSet<Entity>,
) {
    let bodies_without_components: Vec<Entity> = state
        .entity_bodies
        .keys()
        .copied()
        .filter(|entity| !body_owner_entities.contains(entity))
        .collect();

    for entity in bodies_without_components {
        if let Some(collider_handle) = state.entity_colliders.remove(&entity) {
            state.collider_entities.remove(&collider_handle);
        }
        state.transform_cache.remove(&entity);
        state.collider_cache.remove(&entity);
        if let Some(body_handle) = state.entity_bodies.remove(&entity) {
            state.bodies.remove(
                body_handle,
                &mut state.islands,
                &mut state.colliders,
                &mut state.impulse_joints,
                &mut state.multibody_joints,
                true,
            );
        }
    }

    let colliders_without_components: Vec<Entity> = state
        .entity_colliders
        .keys()
        .copied()
        .filter(|entity| !collider_owner_entities.contains(entity))
        .collect();

    for entity in colliders_without_components {
        if let Some(collider_handle) = state.entity_colliders.remove(&entity) {
            state.collider_entities.remove(&collider_handle);
            state.collider_cache.remove(&entity);
            let _ = state.colliders.remove(
                collider_handle,
                &mut state.islands,
                &mut state.bodies,
                true,
            );
        }
    }

    scrub_stale_collider_maps(state, None);
}

pub(crate) fn scrub_stale_collider_maps(state: &mut PhysicsState, live_entities: Option<&HashSet<Entity>>) {
    let stale_reverse: Vec<ColliderHandle> = state
        .collider_entities
        .iter()
        .filter_map(|(collider, entity)| {
            let collider_missing = state.colliders.get(*collider).is_none();
            let entity_dead = live_entities.map(|live| !live.contains(entity)).unwrap_or(false);
            (collider_missing || entity_dead).then_some(*collider)
        })
        .collect();
    for collider in stale_reverse {
        state.collider_entities.remove(&collider);
    }

    let stale_forward: Vec<Entity> = state
        .entity_colliders
        .iter()
        .filter_map(|(entity, collider)| {
            let collider_missing = state.colliders.get(*collider).is_none();
            let entity_dead = live_entities.map(|live| !live.contains(entity)).unwrap_or(false);
            (collider_missing || entity_dead).then_some(*entity)
        })
        .collect();
    for entity in stale_forward {
        state.entity_colliders.remove(&entity);
        state.collider_cache.remove(&entity);
    }
}
