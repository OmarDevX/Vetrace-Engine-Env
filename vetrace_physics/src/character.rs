use glam::Vec3;
use rapier3d::na as nalgebra;
use rapier3d::prelude::{point, vector, QueryFilter, Ray};
use vetrace_core::components::builtins::Transform;
use vetrace_core::engine::Engine;

use crate::components::{
    AngularVelocity, CharacterBody3D, CharacterController3D, CharacterControllerState, Collider,
    ColliderShape, RigidBody3D, Velocity,
};
use crate::state::{isometry_from_pose, PhysicsPose, PhysicsState};

pub(crate) fn prepare_character_bodies(engine: &mut Engine) {
    let bodies: Vec<_> = engine.raw_world().query::<CharacterBody3D>()
        .into_iter()
        .map(|(entity, body)| (entity, body.clone()))
        .collect();

    for (entity, body) in bodies {
        if !engine.raw_world().has::<Transform>(entity) {
            engine.raw_world_mut().insert(entity, Transform::default());
        }
        if !engine.raw_world().has::<RigidBody3D>(entity) {
            engine.raw_world_mut().insert(entity, RigidBody3D::default());
        }
        if !engine.raw_world().has::<Velocity>(entity) {
            engine.raw_world_mut().insert(entity, Velocity::default());
        }
        if !engine.raw_world().has::<AngularVelocity>(entity) {
            engine.raw_world_mut().insert(entity, AngularVelocity::default());
        }
        if !engine.raw_world().has::<Collider>(entity) {
            engine.raw_world_mut().insert(entity, Collider {
                handle: None,
                shape: ColliderShape::Capsule,
                half_extents: Vec3::new(body.radius, body.height * 0.5, body.radius),
                offset: Vec3::ZERO,
                ..Collider::default()
            });
        }
        if !engine.raw_world().has::<CharacterController3D>(entity) {
            engine.raw_world_mut().insert(entity, body.sensor());
        } else if let Some(sensor) = engine.raw_world_mut().get_mut::<CharacterController3D>(entity) {
            *sensor = body.sensor();
        }
        if !engine.raw_world().has::<CharacterControllerState>(entity) {
            engine.raw_world_mut().insert(entity, CharacterControllerState::default());
        }
    }
}

pub(crate) fn apply_character_body_motion(engine: &mut Engine, dt: f32) {
    let bodies: Vec<_> = engine.raw_world().query::<CharacterBody3D>()
        .into_iter()
        .map(|(entity, body)| {
            let state = engine.raw_world().get::<CharacterControllerState>(entity).cloned().unwrap_or_default();
            (entity, body.clone(), state)
        })
        .collect();

    for (entity, body, state) in bodies {
        let mut desired = body.desired_velocity;
        desired.y = 0.0;
        desired = desired.clamp_length_max(body.move_speed.max(0.0));

        if state.grounded && state.ground_normal.y > 0.05 {
            desired = (desired - state.ground_normal * desired.dot(state.ground_normal))
                .clamp_length_max(body.move_speed.max(0.0));
        }

        if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity>(entity) {
            velocity.linear.x = desired.x;
            velocity.linear.z = desired.z;

            let can_jump = state.grounded && state.vertical_speed.abs() <= 0.35;
            if body.jump_requested && can_jump {
                velocity.linear.y = body.jump_speed;
            } else if body.stop_on_ground && state.grounded && velocity.linear.y < 0.0 {
                velocity.linear.y = 0.0;
            } else if velocity.linear.y < -body.max_fall_speed {
                velocity.linear.y = -body.max_fall_speed;
            }
        }

        if let Some(existing) = engine.raw_world_mut().get_mut::<CharacterBody3D>(entity) {
            existing.jump_requested = false;
        }
    }

    // `dt` is currently not needed by the velocity-style controller, but it is
    // kept in the signature so acceleration-based variants can be added without
    // changing the plugin call site.
    let _ = dt;
}

pub(crate) fn update_character_controller_states(engine: &mut Engine) {
    let controllers: Vec<_> = engine.raw_world().query::<CharacterController3D>()
        .into_iter()
        .map(|(entity, controller)| {
            let transform = engine.raw_world().get::<Transform>(entity).cloned().unwrap_or_default();
            let vertical_speed = engine.raw_world().get::<Velocity>(entity).map(|v| v.linear.y).unwrap_or(0.0);
            (entity, controller.clone(), transform, vertical_speed)
        })
        .collect();

    let states: Vec<_> = engine
        .get_resource::<PhysicsState>()
        .map(|state| {
            controllers
                .iter()
                .map(|(entity, controller, transform, vertical_speed)| {
                    let own_collider = state.entity_colliders.get(entity).copied();
                    let ray_origin = transform.translation;
                    let max_distance = controller.height * 0.5 + controller.ground_snap + 0.08;
                    let ray = Ray::new(point![ray_origin.x, ray_origin.y, ray_origin.z], vector![0.0, -1.0, 0.0]);
                    let mut filter = QueryFilter::default();
                    filter.exclude_collider = own_collider;
                    let hit = state.query_pipeline.cast_ray_and_get_normal(
                        &state.bodies,
                        &state.colliders,
                        &ray,
                        max_distance,
                        true,
                        filter,
                    );
                    let mut out = CharacterControllerState { vertical_speed: *vertical_speed, ..CharacterControllerState::default() };
                    if let Some((collider, intersection)) = hit {
                        let raw_normal = Vec3::new(intersection.normal.x, intersection.normal.y, intersection.normal.z);
                        let normal = if raw_normal.length_squared() > 1.0e-8 { raw_normal.normalize() } else { Vec3::Y };
                        let slope = normal.angle_between(Vec3::Y);
                        out.ground_entity = state.collider_entities.get(&collider).copied();
                        out.ground_distance = intersection.time_of_impact;
                        out.ground_normal = normal;
                        out.slope_radians = slope;
                        out.grounded = out.ground_entity != Some(*entity)
                            && intersection.time_of_impact <= max_distance
                            && slope <= controller.max_slope_radians
                            && *vertical_speed >= -0.75
                            && *vertical_speed <= 0.25;
                    }
                    (*entity, out)
                })
                .collect()
        })
        .unwrap_or_default();

    for (entity, state) in states {
        if engine.raw_world().has::<CharacterControllerState>(entity) {
            if let Some(existing) = engine.raw_world_mut().get_mut::<CharacterControllerState>(entity) {
                *existing = state;
            }
        } else {
            engine.raw_world_mut().insert(entity, state);
        }
    }
}

/// Corrects small character-controller penetration after Rapier has solved the
/// frame. Rapier still performs the actual collision response; this helper only
/// keeps the ECS/rigid-body center aligned with the controller capsule when a
/// grounded character is microscopically below the walkable surface.
pub(crate) fn snap_character_bodies_to_ground(engine: &mut Engine) {
    let snap_ops: Vec<_> = engine.raw_world().query::<CharacterBody3D>()
        .into_iter()
        .filter_map(|(entity, body)| {
            if !body.snap_to_ground { return None; }
            let controller = body.sensor();
            let state = engine.raw_world().get::<CharacterControllerState>(entity)?;
            if !state.grounded || !state.ground_distance.is_finite() {
                return None;
            }

            // The ray starts at the character body center. For the simplified
            // FPS capsule used by the active backend, the expected distance from
            // center to walkable ground is half the controller height.
            let expected_distance = controller.height * 0.5;
            let correction_y = expected_distance - state.ground_distance;

            // Only raise characters out of penetration, and do it in small
            // increments.  The old cap used ground_snap + step_height, which can
            // allow a visible 30-50cm pop if Rapier lets the capsule sink for a
            // frame during jump/landing.  A tiny correction keeps the controller
            // stable without camera/mesh teleporting upward.
            let max_correction = 0.025_f32;
            if correction_y > 0.0005 {
                Some((entity, correction_y.min(max_correction)))
            } else {
                None
            }
        })
        .collect();

    if snap_ops.is_empty() {
        return;
    }

    let mut body_updates = Vec::new();
    for (entity, correction_y) in snap_ops {
        let mut new_pose = None;
        if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
            transform.translation.y += correction_y;
            new_pose = Some(PhysicsPose::from_transform(transform));
        }

        if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity>(entity) {
            if velocity.linear.y < 0.0 {
                velocity.linear.y = 0.0;
            }
        }

        if let Some(pose) = new_pose {
            body_updates.push((entity, pose));
        }
    }

    if body_updates.is_empty() {
        return;
    }

    if let Some(physics) = engine.get_resource_mut::<PhysicsState>() {
        for (entity, pose) in body_updates {
            if let Some(handle) = physics.entity_bodies.get(&entity).copied() {
                if let Some(body) = physics.bodies.get_mut(handle) {
                    body.set_position(isometry_from_pose(pose), true);
                    physics.transform_cache.insert(entity, pose);
                    let mut linvel = *body.linvel();
                    if linvel.y < 0.0 {
                        linvel.y = 0.0;
                        body.set_linvel(linvel, true);
                    }
                }
            }
        }
        physics.query_pipeline.update(&physics.colliders);
    }
}
