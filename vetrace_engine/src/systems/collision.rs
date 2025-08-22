use crate::behaviour::script::ScriptBehaviour;
use crate::components::components::{
    Collider, ObjectRef, RigidBody3D, ScriptComponent, StaticBody, Transform, Velocity,
};
use crate::ecs::{Behaviour, Entity};
use crate::engine::engine::Engine;
use rapier3d::prelude::*;

#[derive(Clone, Copy)]
pub struct CollisionEvent {
    pub a: Entity,
    pub b: Entity,
}

pub struct CollisionSystem;

impl CollisionSystem {
    fn scaled_size(engine: &Engine, entity: Entity, size: [f32; 3]) -> [f32; 3] {
        if let Some(obj_ref) = engine.world.get::<ObjectRef>(entity) {
            if let Some(obj) = engine.scene.objects.get(obj_ref.id as usize) {
                return [
                    obj.scale[0] * obj.size[0],
                    obj.scale[1] * obj.size[1],
                    obj.scale[2] * obj.size[2],
                ];
            }
        }
        size
    }
    fn handle_lua(engine: &mut Engine, event: CollisionEvent) {
        for &entity in &[event.a, event.b] {
            if let Some(script) = engine.get_component_mut_entity::<ScriptComponent>(entity) {
                let name = script.script.clone();
                if let Some(beh) = engine.script_library.get(&name) {
                    let ptr = beh as *const ScriptBehaviour;
                    let other = if entity == event.a { event.b } else { event.a };
                    unsafe {
                        (*ptr).on_collision(engine, entity.0, other.0);
                    }
                }
            }
        }
    }

    fn intersects(
        engine: &Engine,
        e1: Entity,
        c1: &Collider,
        t1: &Transform,
        e2: Entity,
        c2: &Collider,
        t2: &Transform,
    ) -> bool {
        let iso1 = t1.iso();
        let iso2 = t2.iso();
        let size1 = Self::scaled_size(engine, e1, t1.size);
        let size2 = Self::scaled_size(engine, e2, t2.size);
        let s1 = c1.shape(size1);
        let s2 = c2.shape(size2);
        rapier3d::parry::query::intersection_test(&iso1, &*s1, &iso2, &*s2).unwrap_or(false)
    }

    fn penetration_vector(
        engine: &Engine,
        static_e: Entity,
        c_static: &Collider,
        t_static: &Transform,
        dynamic_e: Entity,
        c_dyn: &Collider,
        t_dyn: &Transform,
    ) -> Option<Vector<Real>> {
        let iso1 = t_static.iso();
        let iso2 = t_dyn.iso();
        let size_static = Self::scaled_size(engine, static_e, t_static.size);
        let size_dyn = Self::scaled_size(engine, dynamic_e, t_dyn.size);
        let s1 = c_static.shape(size_static);
        let s2 = c_dyn.shape(size_dyn);
        if let Ok(Some(contact)) = rapier3d::parry::query::contact(&iso1, &*s1, &iso2, &*s2, 0.0) {
            Some(contact.normal1.into_inner() * -contact.dist)
        } else {
            None
        }
    }

    fn resolve_static(engine: &mut Engine, static_e: Entity, dynamic_e: Entity) {
        let (s_col, s_t) = match (
            engine.world.get::<Collider>(static_e),
            engine.world.get::<Transform>(static_e),
        ) {
            (Some(c), Some(t)) => (c.clone(), *t),
            _ => return,
        };

        let (d_col, mut d_t) = match (
            engine.world.get::<Collider>(dynamic_e),
            engine.world.get::<Transform>(dynamic_e),
        ) {
            (Some(c), Some(t)) => (c.clone(), *t),
            _ => return,
        };

        if let Some(pen) =
            Self::penetration_vector(engine, static_e, &s_col, &s_t, dynamic_e, &d_col, &d_t)
        {
            d_t.position[0] += pen.x;
            d_t.position[1] += pen.y;
            d_t.position[2] += pen.z;
            if let Some(t) = engine.world.get_mut::<Transform>(dynamic_e) {
                t.position = d_t.position;
            }
            if let Some(v) = engine.world.get_mut::<Velocity>(dynamic_e) {
                v.velocity = [0.0; 3];
            }
            if let Some(rb) = engine.world.get::<RigidBody3D>(dynamic_e) {
                if let Some(handle) = rb.handle {
                    if let Some(body) = engine.physics.bodies.get_mut(handle) {
                        let trans = body.translation() + pen;
                        body.set_translation(trans, true);
                        body.set_linvel(vector![0.0, 0.0, 0.0], true);
                    }
                }
            }
        }
    }
}

impl Behaviour for CollisionSystem {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        engine.collision_events.clear();
        let entities = engine.world.entities().to_vec();
        for i in 0..entities.len() {
            for j in (i + 1)..entities.len() {
                let e1 = entities[i];
                let e2 = entities[j];
                if let (Some(c1), Some(t1), Some(c2), Some(t2)) = (
                    engine.world.get::<Collider>(e1),
                    engine.world.get::<Transform>(e1),
                    engine.world.get::<Collider>(e2),
                    engine.world.get::<Transform>(e2),
                ) {
                    let rb1 = engine.world.get::<RigidBody3D>(e1);
                    let rb2 = engine.world.get::<RigidBody3D>(e2);
                    let s1 = engine.world.has::<StaticBody>(e1);
                    let s2 = engine.world.has::<StaticBody>(e2);
                    if Self::intersects(engine, e1, c1, t1, e2, c2, t2) {
                        let ev = CollisionEvent { a: e1, b: e2 };
                        engine.collision_events.push(ev);
                        engine.collision_event.emit(ev);
                        if rb1.is_none() && rb2.is_none() {
                            if s1 && !s2 {
                                Self::resolve_static(engine, e1, e2);
                            } else if s2 && !s1 {
                                Self::resolve_static(engine, e2, e1);
                            }
                        }
                    }
                }
            }
        }
        let events = engine.collision_events.clone();
        for ev in events {
            Self::handle_lua(engine, ev);
        }
    }
}