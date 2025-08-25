use glam::{Quat, Vec3};
use std::collections::HashSet;

use crate::{
    components::components::{AngularVelocity, Transform, Velocity, RigidBody3D},
    engine::engine::Engine,
    Behaviour,
};

/// System that updates [`Transform`] components based on velocity data.
///
/// Entities with a [`Velocity`] component have their position advanced each
/// frame. Entities with an [`AngularVelocity`] component have their orientation
/// updated using simple quaternion integration.
pub struct TransformSyncSystem;

impl Behaviour for TransformSyncSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        // Gather entities controlled by Rapier so we avoid double integration.
        let rapier_entities: HashSet<_> = engine
            .world
            .query_mut::<RigidBody3D>()
            .iter()
            .map(|(e, _)| *e)
            .collect();

        // Integrate acceleration into velocity for entities without a rapier body
        for (e, vel) in engine.world.query_mut::<Velocity>() {
            if rapier_entities.contains(&e) {
                continue;
            }
            vel.velocity[0] += vel.acceleration[0] * delta;
            vel.velocity[1] += vel.acceleration[1] * delta;
            vel.velocity[2] += vel.acceleration[2] * delta;
        }

        // Apply gravity to velocity when requested
        for (_e, _vel, _rb) in engine.world.query2_mut::<Velocity, RigidBody3D>() {
            // handled by rapier
        }

        // Apply linear velocity
        for (e, transform, vel) in engine.world.query2_mut::<Transform, Velocity>() {
            if rapier_entities.contains(&e) {
                continue;
            }
            transform.position[0] += vel.velocity[0] * delta;
            transform.position[1] += vel.velocity[1] * delta;
            transform.position[2] += vel.velocity[2] * delta;
        }

        // Integrate angular acceleration
        for (e, ang) in engine.world.query_mut::<AngularVelocity>() {
            if rapier_entities.contains(&e) {
                continue;
            }
            ang.angular_velocity[0] += ang.angular_acceleration[0] * delta;
            ang.angular_velocity[1] += ang.angular_acceleration[1] * delta;
            ang.angular_velocity[2] += ang.angular_acceleration[2] * delta;
        }

        // Apply angular velocity
        for (e, transform, ang) in engine.world.query2_mut::<Transform, AngularVelocity>() {
            if rapier_entities.contains(&e) {
                continue;
            }
            let axis = Vec3::from(ang.angular_velocity);
            let speed = axis.length();
            if speed != 0.0 {
                let rot = Quat::from_axis_angle(axis.normalize(), speed * delta);
                let current = Quat::from_xyzw(
                    transform.orientation[0],
                    transform.orientation[1],
                    transform.orientation[2],
                    transform.orientation[3],
                );
                let new_q = rot * current;
                transform.orientation = [new_q.x, new_q.y, new_q.z, new_q.w];
            }
        }
    }
}

