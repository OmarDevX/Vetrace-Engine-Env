use rapier3d::na::{Quaternion, Translation3 as Translation, Unit, UnitQuaternion};
use rapier3d::prelude::*;

use crate::{
    components::components::{
        AngularVelocity, BallJoint, Collider, ColliderShape, KinematicBody, RevoluteJoint,
        RigidBody3D, StaticBody, Transform, Velocity,
    },
    engine::engine::Engine,
    Behaviour,
};

pub struct RapierPhysicsSystem;

impl Behaviour for RapierPhysicsSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut rb_missing = Vec::new();
        for (e, t, c, rb) in engine
            .world
            .query3_mut::<Transform, Collider, RigidBody3D>()
        {
            if rb.handle.is_none() {
                rb_missing.push((e, *t, *c));
            }
        }

        let mut static_only = Vec::new();
        for (e, t, c, sb) in engine.world.query3_mut::<Transform, Collider, StaticBody>() {
            if sb.handle.is_none() {
                static_only.push((e, *t, *c));
            }
        }
        let mut kinematic_only = Vec::new();
        for (e, t, c, kb) in engine
            .world
            .query3_mut::<Transform, Collider, KinematicBody>()
        {
            if kb.handle.is_none() {
                kinematic_only.push((e, *t, *c));
            }
        }

        enum Kind {
            Dynamic,
            Static,
            Kinematic,
        }
        let mut to_add = Vec::new();
        for (e, t, c) in rb_missing {
            to_add.push((e, t, c, Kind::Dynamic));
        }

        for (e, t, c) in static_only {
            if engine.world.get::<RigidBody3D>(e).is_none() {
                to_add.push((e, t, c, Kind::Static));
            }
        }
        for (e, t, c) in kinematic_only {
            if engine.world.get::<RigidBody3D>(e).is_none()
                && engine.world.get::<StaticBody>(e).is_none()
            {
                to_add.push((e, t, c, Kind::Kinematic));
            }
        }

        for (e, t, c, kind) in to_add {
            let iso = t.iso();
            let mut builder = match kind {
                Kind::Static => RigidBodyBuilder::fixed().position(iso),
                Kind::Dynamic => RigidBodyBuilder::dynamic().position(iso).ccd_enabled(true),
                Kind::Kinematic => RigidBodyBuilder::kinematic_position_based().position(iso),
            };
            if let Some(rb) = engine.world.get::<RigidBody3D>(e) {
                if !rb.gravity_enabled {
                    builder = builder.gravity_scale(0.0);
                }
                if !matches!(kind, Kind::Static) {
                    builder = builder
                        .additional_mass(rb.mass)
                        .linear_damping(rb.linear_damping)
                        .angular_damping(rb.angular_damping);
                }
            }
            let handle = engine.physics.bodies.insert(builder.build());

            let mut col_builder = match c.shape {
                ColliderShape::Sphere => ColliderBuilder::ball(c.size[0] * 0.5),
                ColliderShape::Cube => {
                    ColliderBuilder::cuboid(c.size[0] * 0.5, c.size[1] * 0.5, c.size[2] * 0.5)
                }
                ColliderShape::Capsule => {
                    let radius = c.size[0] * 0.5;
                    let half_height = (c.size[1] * 0.5 - radius).max(0.0);
                    ColliderBuilder::capsule_y(half_height, radius)
                }
            };
            col_builder = col_builder.position(Isometry::from_parts(
                Translation::new(c.position[0], c.position[1], c.position[2]),
                UnitQuaternion::from_quaternion(Quaternion::new(
                    c.rotation[3],
                    c.rotation[0],
                    c.rotation[1],
                    c.rotation[2],
                )),
            ));
            if let Some(rb) = engine.world.get::<RigidBody3D>(e) {
                col_builder = col_builder.friction(rb.friction).restitution(rb.bounciness);
            } else {
                col_builder = col_builder.friction(1.0);
            }
            engine.physics.colliders.insert_with_parent(
                col_builder.build(),
                handle,
                &mut engine.physics.bodies,
            );
            if let Some(rb_comp) = engine.world.get_mut::<RigidBody3D>(e) {
                rb_comp.handle = Some(handle);
            } else if let Some(sb_comp) = engine.world.get_mut::<StaticBody>(e) {
                sb_comp.handle = Some(handle);
            } else if let Some(kb_comp) = engine.world.get_mut::<KinematicBody>(e) {
                kb_comp.handle = Some(handle);
            }
        }

        let mut pending_joints = Vec::new();
        for (e, joint) in engine.world.query_mut::<RevoluteJoint>() {
            if joint.handle.is_none() {
                pending_joints.push((e, joint.other, joint.axis));
            }
        }

        let mut pending_balls = Vec::new();
        for (e, joint) in engine.world.query_mut::<BallJoint>() {
            if joint.handle.is_none() {
                pending_balls.push((e, joint.other));
            }
        }

        for (e, other_id, axis) in pending_joints {
            if let Some(other_e) = engine.core.find_entity_by_object_id(other_id) {
                let get_handle = |ent| {
                    engine
                        .world
                        .get::<RigidBody3D>(ent)
                        .and_then(|rb| rb.handle)
                        .or_else(|| engine.world.get::<StaticBody>(ent).and_then(|sb| sb.handle))
                        .or_else(|| {
                            engine
                                .world
                                .get::<KinematicBody>(ent)
                                .and_then(|kb| kb.handle)
                        })
                };
                let h1 = get_handle(e);
                let h2 = get_handle(other_e);
                if let (Some(a), Some(b)) = (h1, h2) {
                    let t1 = engine.world.get::<Transform>(e).unwrap();
                    let t2 = engine.world.get::<Transform>(other_e).unwrap();
                    let anchor = vector![
                        t1.position[0] - t2.position[0],
                        t1.position[1] - t2.position[1],
                        t1.position[2] - t2.position[2],
                    ];
                    let axis_v = vector![axis[0], axis[1], axis[2]];
                    let contacts = engine
                        .world
                        .get::<RevoluteJoint>(e)
                        .map(|j| j.contacts_enabled)
                        .unwrap_or(true);
                    let builder = RevoluteJointBuilder::new(Unit::new_normalize(axis_v))
                        .local_anchor2(anchor.into())
                        .contacts_enabled(contacts);
                    let handle = engine.physics.joints.insert(a, b, builder, true);
                    if let Some(j) = engine.world.get_mut::<RevoluteJoint>(e) {
                        j.handle = Some(handle);
                    }
                }
            }
        }

        for (e, other_id) in pending_balls {
            if let Some(other_e) = engine.core.find_entity_by_object_id(other_id) {
                let get_handle = |ent| {
                    engine
                        .world
                        .get::<RigidBody3D>(ent)
                        .and_then(|rb| rb.handle)
                        .or_else(|| engine.world.get::<StaticBody>(ent).and_then(|sb| sb.handle))
                        .or_else(|| {
                            engine
                                .world
                                .get::<KinematicBody>(ent)
                                .and_then(|kb| kb.handle)
                        })
                };
                let h1 = get_handle(e);
                let h2 = get_handle(other_e);
                if let (Some(a), Some(b)) = (h1, h2) {
                    let t1 = engine.world.get::<Transform>(e).unwrap();
                    let t2 = engine.world.get::<Transform>(other_e).unwrap();
                    let anchor = vector![
                        t1.position[0] - t2.position[0],
                        t1.position[1] - t2.position[1],
                        t1.position[2] - t2.position[2],
                    ];
                    let contacts = engine
                        .world
                        .get::<BallJoint>(e)
                        .map(|j| j.contacts_enabled)
                        .unwrap_or(true);
                    let builder = SphericalJointBuilder::new()
                        .local_anchor1(Point::origin())
                        .local_anchor2(anchor.into())
                        .contacts_enabled(contacts);
                    let handle = engine.physics.joints.insert(a, b, builder, true);
                    if let Some(j) = engine.world.get_mut::<BallJoint>(e) {
                        j.handle = Some(handle);
                    }
                }
            }
        }

        for (_e, rb) in engine.world.query_mut::<RigidBody3D>() {
            if let Some(handle) = rb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    body.enable_ccd(true);
                    let scale = if rb.gravity_enabled { 1.0 } else { 0.0 };
                    body.set_gravity_scale(scale, true);
                    body.set_additional_mass(rb.mass, true);
                    body.set_linear_damping(rb.linear_damping);
                    body.set_angular_damping(rb.angular_damping);

                    for ch in body.colliders() {
                        if let Some(c) = engine.physics.colliders.get_mut(*ch) {
                            c.set_friction(rb.friction);
                            c.set_restitution(rb.bounciness);
                        }
                    }
                }
            }
        }

        // Update Rapier bodies if the transform was externally modified
        for (_e, t, rb) in engine.world.query2_mut::<Transform, RigidBody3D>() {
            if let Some(handle) = rb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let iso = t.iso();
                    if *body.position() != iso {
                        body.set_position(iso, true);
                    }
                }
            }
        }
        for (_e, t, sb) in engine.world.query2_mut::<Transform, StaticBody>() {
            if let Some(handle) = sb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let iso = t.iso();
                    if *body.position() != iso {
                        body.set_position(iso, true);
                    }
                }
            }
        }
        for (_e, t, kb) in engine.world.query2_mut::<Transform, KinematicBody>() {
            if let Some(handle) = kb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let iso = t.iso();
                    if *body.position() != iso {
                        body.set_position(iso, true);
                    }
                }
            }
        }

        // Apply user-defined angular velocities and update collider sizes.
        for (_e, _t, av, rb, col) in engine
            .world
            .query4_mut::<Transform, AngularVelocity, RigidBody3D, Collider>()
        {
            if let Some(handle) = rb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let torque = vector![
                        av.angular_acceleration[0],
                        av.angular_acceleration[1],
                        av.angular_acceleration[2],
                    ];
                    // Apply torque each frame without accumulating
                    body.add_torque(torque, true);

                    // Resize collider if the transform scale changed.
                    for ch in body.colliders() {
                        if let Some(col_inst) = engine.physics.colliders.get_mut(*ch) {
                            let shape = match col.shape {
                                ColliderShape::Sphere => SharedShape::ball(col.size[0] * 0.5),
                                ColliderShape::Cube => SharedShape::cuboid(
                                    col.size[0] * 0.5,
                                    col.size[1] * 0.5,
                                    col.size[2] * 0.5,
                                ),
                                ColliderShape::Capsule => {
                                    let radius = col.size[0] * 0.5;
                                    let half_height = (col.size[1] * 0.5 - radius).max(0.0);
                                    SharedShape::capsule_y(half_height, radius)
                                }
                            };
                            col_inst.set_shape(shape);
                        }
                    }
                }
            }
        }
        for (_e, _t, av, kb, col) in engine
            .world
            .query4_mut::<Transform, AngularVelocity, KinematicBody, Collider>()
        {
            if let Some(handle) = kb.handle {
                if let Some(body) = engine.physics.bodies.get_mut(handle) {
                    let torque = vector![
                        av.angular_acceleration[0],
                        av.angular_acceleration[1],
                        av.angular_acceleration[2],
                    ];
                    // Apply torque each frame for kinematic bodies
                    body.add_torque(torque, true);

                    for ch in body.colliders() {
                        if let Some(col_inst) = engine.physics.colliders.get_mut(*ch) {
                            let shape = match col.shape {
                                ColliderShape::Sphere => SharedShape::ball(col.size[0] * 0.5),
                                ColliderShape::Cube => SharedShape::cuboid(
                                    col.size[0] * 0.5,
                                    col.size[1] * 0.5,
                                    col.size[2] * 0.5,
                                ),
                                ColliderShape::Capsule => {
                                    let radius = col.size[0] * 0.5;
                                    let half_height = (col.size[1] * 0.5 - radius).max(0.0);
                                    SharedShape::capsule_y(half_height, radius)
                                }
                            };
                            col_inst.set_shape(shape);
                        }
                    }
                }
            }
        }

        engine.physics.integration_parameters.dt = delta;
        engine.physics.pipeline.step(
            &engine.physics.gravity,
            &engine.physics.integration_parameters,
            &mut engine.physics.island_manager,
            &mut engine.physics.broad_phase,
            &mut engine.physics.narrow_phase,
            &mut engine.physics.bodies,
            &mut engine.physics.colliders,
            &mut engine.physics.joints,
            &mut engine.physics.multibody_joints,
            &mut engine.physics.ccd_solver,
            None,
            &(),
            &(),
        );

        let mut vel_updates = Vec::new();
        for (e, t, rb) in engine.world.query2_mut::<Transform, RigidBody3D>() {
            if let Some(handle) = rb.handle {
                if let Some(body) = engine.physics.bodies.get(handle) {
                    let pos = body.position();
                    t.position = [pos.translation.x, pos.translation.y, pos.translation.z];
                    let rot = pos.rotation;
                    t.orientation = [rot.i, rot.j, rot.k, rot.w];
                    if body.is_dynamic() {
                        vel_updates.push((
                            e,
                            body.linvel(),
                            body.angvel(),
                            rb.gravity_enabled,
                            rb.gravity_force,
                        ));
                    }
                }
            }
        }
        for (_e, t, kb) in engine.world.query2_mut::<Transform, KinematicBody>() {
            if let Some(handle) = kb.handle {
                if let Some(body) = engine.physics.bodies.get(handle) {
                    let pos = body.position();
                    t.position = [pos.translation.x, pos.translation.y, pos.translation.z];
                    let rot = pos.rotation;
                    t.orientation = [rot.i, rot.j, rot.k, rot.w];
                }
            }
        }

        for (e, linvel, angvel, use_gravity, gravity_force) in vel_updates {
            let gravity = if use_gravity {
                gravity_force
            } else {
                [0.0, 0.0, 0.0]
            };
            if let Some(v) = engine.world.get_mut::<Velocity>(e) {
                v.velocity = [linvel.x, linvel.y, linvel.z];
                v.acceleration = gravity;
            }
            if let Some(av) = engine.world.get_mut::<AngularVelocity>(e) {
                av.angular_velocity = [angvel.x, angvel.y, angvel.z];
            }
        }
    }
}
