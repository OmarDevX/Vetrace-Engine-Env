use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;
use vetrace_core::{Engine, Entity};

use super::events::{CollisionContact2D, CollisionStarted2D, CollisionStopped2D};
use super::geometry::{collide, ContactManifold2D, WorldShape2D};
use super::state::{ActivePair2D, PairKey2D, Physics2dStats, Physics2dState};
use super::transforms::{current_physics_transform_2d, write_world_pose_2d};
use super::{BodyType2D, Collider2D, RigidBody2D, Velocity2D};

#[derive(Clone, Debug)]
struct BodySnapshot2D {
    entity: Entity,
    body: RigidBody2D,
    collider: Collider2D,
    position: Vec2,
    rotation: f32,
    scale: Vec2,
    velocity: Vec2,
    angular_velocity: f32,
    has_body_component: bool,
    has_velocity_component: bool,
}

impl BodySnapshot2D {
    fn shape(&self) -> WorldShape2D {
        WorldShape2D::from_collider(self.position, self.rotation, self.scale, &self.collider)
    }

    fn inverse_mass(&self) -> f32 {
        if self.body.body_type == BodyType2D::Dynamic {
            1.0 / self.body.mass.max(0.0001)
        } else {
            0.0
        }
    }

    fn integrates(&self) -> bool {
        matches!(self.body.body_type, BodyType2D::Dynamic | BodyType2D::Kinematic)
    }
}

pub(crate) fn step_physics_2d(state: &mut Physics2dState, engine: &mut Engine, dt: f32) {
    let mut bodies = collect_bodies(engine);
    let dt = if dt.is_finite() { dt.clamp(0.0, 1.0 / 10.0) } else { 0.0 };
    let gravity = finite_vec2(state.gravity);
    let solver_iterations = state.solver_iterations.clamp(1, 32);
    let substeps = choose_substeps(state, &bodies, dt);
    let step_dt = if substeps > 0 { dt / substeps as f32 } else { 0.0 };
    let mut stats = Physics2dStats { bodies: bodies.len(), substeps, ..Physics2dStats::default() };

    for _ in 0..substeps {
        integrate_bodies(&mut bodies, gravity, step_dt);
        let candidates = broadphase_pairs(&bodies, state.broadphase_cell_size);
        stats.broadphase_pairs = stats.broadphase_pairs.saturating_add(candidates.len());

        let contacts = detect_contacts(&bodies, &candidates);
        stats.contacts = stats.contacts.saturating_add(contacts.len());
        publish_pair_events(state, engine, &bodies, &contacts);

        for _ in 0..solver_iterations {
            let iteration_contacts = detect_contacts(&bodies, &candidates);
            if iteration_contacts.is_empty() { break; }
            for (a, b, manifold) in iteration_contacts {
                if bodies[a].collider.sensor || bodies[b].collider.sensor { continue; }
                resolve_contact(&mut bodies, a, b, manifold);
            }
        }
    }

    if substeps == 0 {
        // Keep started/stopped state correct even while paused or on a zero-delta editor frame.
        let candidates = broadphase_pairs(&bodies, state.broadphase_cell_size);
        let contacts = detect_contacts(&bodies, &candidates);
        stats.broadphase_pairs = candidates.len();
        stats.contacts = contacts.len();
        publish_pair_events(state, engine, &bodies, &contacts);
    }

    write_back(engine, &bodies);
    state.stats = stats;
    engine.profile_record_counter("physics.2d.bodies", stats.bodies as f64, "bodies");
    engine.profile_record_counter("physics.2d.broadphase_pairs", stats.broadphase_pairs as f64, "pairs");
    engine.profile_record_counter("physics.2d.contacts", stats.contacts as f64, "contacts");
    engine.profile_record_counter("physics.2d.substeps", stats.substeps as f64, "steps");
}

fn collect_bodies(engine: &Engine) -> Vec<BodySnapshot2D> {
    let mut bodies = engine
        .raw_world()
        .query::<Collider2D>()
        .into_iter()
        .filter_map(|(entity, collider)| {
            if !collider.enabled { return None; }
            if engine.raw_world().get::<vetrace_core::Transform>(entity).is_none() { return None; }
            let transform = current_physics_transform_2d(engine, entity);
            let body_component = engine.raw_world().get::<RigidBody2D>(entity).cloned();
            let body = sanitize_body(body_component.clone().unwrap_or_else(RigidBody2D::static_body));
            if !body.enabled { return None; }
            let velocity_component = engine.raw_world().get::<Velocity2D>(entity).cloned();
            let velocity = velocity_component.clone().unwrap_or_default();
            Some(BodySnapshot2D {
                entity,
                body,
                collider: sanitize_collider((*collider).clone()),
                position: transform.position,
                rotation: transform.rotation,
                scale: transform.scale,
                velocity: finite_vec2(velocity.linear),
                angular_velocity: if velocity.angular.is_finite() { velocity.angular } else { 0.0 },
                has_body_component: body_component.is_some(),
                has_velocity_component: velocity_component.is_some(),
            })
        })
        .collect::<Vec<_>>();
    bodies.sort_by_key(|body| body.entity);
    bodies
}

fn choose_substeps(state: &Physics2dState, bodies: &[BodySnapshot2D], dt: f32) -> usize {
    if dt <= 0.0 { return 0; }
    let mut requested = 1usize;
    for body in bodies.iter().filter(|body| body.body.continuous && body.integrates()) {
        let distance = body.velocity.length() * dt;
        let allowed = (body.shape().minimum_extent() * 0.5).max(0.025);
        requested = requested.max((distance / allowed).ceil() as usize);
    }
    requested.clamp(1, state.max_substeps.clamp(1, 64))
}

fn integrate_bodies(bodies: &mut [BodySnapshot2D], gravity: Vec2, dt: f32) {
    if dt <= 0.0 { return; }
    for body in bodies {
        if !body.integrates() { continue; }
        if body.body.body_type == BodyType2D::Dynamic {
            body.velocity += gravity * body.body.gravity_scale * dt;
        }
        let linear_decay = 1.0 / (1.0 + body.body.linear_damping.max(0.0) * dt);
        let angular_decay = 1.0 / (1.0 + body.body.angular_damping.max(0.0) * dt);
        body.velocity *= linear_decay;
        body.angular_velocity *= angular_decay;
        body.position += body.velocity * dt;
        if body.body.lock_rotation {
            body.angular_velocity = 0.0;
        } else {
            body.rotation += body.angular_velocity * dt;
        }
    }
}

fn broadphase_pairs(bodies: &[BodySnapshot2D], requested_cell_size: f32) -> BTreeSet<(usize, usize)> {
    let cell_size = if requested_cell_size.is_finite() { requested_cell_size.max(0.05) } else { 2.0 };
    let mut cells: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
    let mut large = Vec::new();

    for (index, body) in bodies.iter().enumerate() {
        let aabb = body.shape().aabb();
        let min_x = floor_cell(aabb.min.x, cell_size);
        let min_y = floor_cell(aabb.min.y, cell_size);
        let max_x = floor_cell(aabb.max.x, cell_size);
        let max_y = floor_cell(aabb.max.y, cell_size);
        let width = (max_x as i64 - min_x as i64 + 1).max(0) as usize;
        let height = (max_y as i64 - min_y as i64 + 1).max(0) as usize;
        if width.saturating_mul(height) > 256 {
            large.push(index);
            continue;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                cells.entry((x, y)).or_default().push(index);
            }
        }
    }

    let mut pairs = BTreeSet::new();
    for indices in cells.values() {
        for left in 0..indices.len() {
            for right in (left + 1)..indices.len() {
                insert_pair(&mut pairs, indices[left], indices[right]);
            }
        }
    }
    for &large_index in &large {
        for other in 0..bodies.len() {
            if large_index != other { insert_pair(&mut pairs, large_index, other); }
        }
    }
    pairs
}

fn detect_contacts(
    bodies: &[BodySnapshot2D],
    candidates: &BTreeSet<(usize, usize)>,
) -> Vec<(usize, usize, ContactManifold2D)> {
    let mut contacts = Vec::new();
    for &(a, b) in candidates {
        let body_a = &bodies[a];
        let body_b = &bodies[b];
        if !layers_interact(&body_a.collider, &body_b.collider) { continue; }
        if body_a.body.body_type == BodyType2D::Static
            && body_b.body.body_type == BodyType2D::Static
            && !body_a.collider.sensor
            && !body_b.collider.sensor
        {
            continue;
        }
        let shape_a = body_a.shape();
        let shape_b = body_b.shape();
        if !shape_a.aabb().overlaps(shape_b.aabb()) { continue; }
        if let Some(contact) = collide(shape_a, shape_b) {
            contacts.push((a, b, contact));
        }
    }
    contacts
}

fn publish_pair_events(
    state: &mut Physics2dState,
    engine: &mut Engine,
    bodies: &[BodySnapshot2D],
    contacts: &[(usize, usize, ContactManifold2D)],
) {
    let mut current = BTreeMap::new();
    for &(a, b, manifold) in contacts {
        let entity_a = bodies[a].entity;
        let entity_b = bodies[b].entity;
        let key = PairKey2D::new(entity_a, entity_b);
        let sensor = bodies[a].collider.sensor || bodies[b].collider.sensor;
        current.insert(key, ActivePair2D { sensor });

        engine.send_event(CollisionContact2D {
            entity_a,
            entity_b,
            sensor,
            normal: manifold.normal,
            point: manifold.point,
            penetration: manifold.penetration,
        });
        if !state.active_pairs.contains_key(&key) {
            engine.send_event(CollisionStarted2D {
                entity_a,
                entity_b,
                sensor,
                normal: manifold.normal,
                point: manifold.point,
                penetration: manifold.penetration,
            });
        }
    }

    for (key, previous) in &state.active_pairs {
        if !current.contains_key(key) {
            engine.send_event(CollisionStopped2D {
                entity_a: key.0,
                entity_b: key.1,
                sensor: previous.sensor,
            });
        }
    }
    state.active_pairs = current;
}

fn resolve_contact(
    bodies: &mut [BodySnapshot2D],
    a: usize,
    b: usize,
    contact: ContactManifold2D,
) {
    let (body_a, body_b) = two_mut(bodies, a, b);
    let inv_a = body_a.inverse_mass();
    let inv_b = body_b.inverse_mass();
    let inverse_mass_sum = inv_a + inv_b;
    if inverse_mass_sum <= 0.0 { return; }

    let normal = contact.normal.normalize_or_zero();
    if normal == Vec2::ZERO { return; }

    let correction_magnitude = ((contact.penetration - 0.001).max(0.0) * 0.72) / inverse_mass_sum;
    let correction = normal * correction_magnitude;
    body_a.position -= correction * inv_a;
    body_b.position += correction * inv_b;

    let relative_velocity = body_b.velocity - body_a.velocity;
    let velocity_along_normal = relative_velocity.dot(normal);
    if velocity_along_normal >= 0.0 { return; }

    let restitution = body_a.collider.restitution.min(body_b.collider.restitution).clamp(0.0, 1.0);
    let impulse_magnitude = -(1.0 + restitution) * velocity_along_normal / inverse_mass_sum;
    let impulse = normal * impulse_magnitude;
    body_a.velocity -= impulse * inv_a;
    body_b.velocity += impulse * inv_b;

    let relative_after = body_b.velocity - body_a.velocity;
    let tangent = (relative_after - normal * relative_after.dot(normal)).normalize_or_zero();
    if tangent == Vec2::ZERO { return; }
    let friction_impulse = -relative_after.dot(tangent) / inverse_mass_sum;
    let friction = (body_a.collider.friction.max(0.0) * body_b.collider.friction.max(0.0)).sqrt();
    let clamped = friction_impulse.clamp(-impulse_magnitude * friction, impulse_magnitude * friction);
    let tangent_impulse = tangent * clamped;
    body_a.velocity -= tangent_impulse * inv_a;
    body_b.velocity += tangent_impulse * inv_b;
}

fn write_back(engine: &mut Engine, bodies: &[BodySnapshot2D]) {
    for body in bodies {
        if !body.has_body_component || body.body.body_type == BodyType2D::Static { continue; }
        write_world_pose_2d(engine, body.entity, body.position, body.rotation);
        if body.has_velocity_component {
            if let Some(velocity) = engine.raw_world_mut().get_mut::<Velocity2D>(body.entity) {
                velocity.linear = body.velocity;
                velocity.angular = body.angular_velocity;
            }
        } else {
            engine.raw_world_mut().insert(
                body.entity,
                Velocity2D { linear: body.velocity, angular: body.angular_velocity },
            );
        }
    }
}

fn layers_interact(a: &Collider2D, b: &Collider2D) -> bool {
    a.collision_layer & b.collision_mask != 0 && b.collision_layer & a.collision_mask != 0
}

fn insert_pair(pairs: &mut BTreeSet<(usize, usize)>, a: usize, b: usize) {
    if a < b { pairs.insert((a, b)); } else if b < a { pairs.insert((b, a)); }
}

fn floor_cell(value: f32, cell_size: f32) -> i32 {
    let scaled = (value / cell_size).floor();
    if !scaled.is_finite() {
        0
    } else {
        scaled.clamp(i32::MIN as f32, i32::MAX as f32) as i32
    }
}

fn two_mut<T>(slice: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    debug_assert_ne!(a, b);
    if a < b {
        let (left, right) = slice.split_at_mut(b);
        (&mut left[a], &mut right[0])
    } else {
        let (left, right) = slice.split_at_mut(a);
        (&mut right[0], &mut left[b])
    }
}



fn sanitize_body(mut body: RigidBody2D) -> RigidBody2D {
    body.mass = finite_or(body.mass, 1.0).max(0.0001);
    body.gravity_scale = finite_or(body.gravity_scale, 0.0);
    body.linear_damping = finite_or(body.linear_damping, 0.0).max(0.0);
    body.angular_damping = finite_or(body.angular_damping, 0.0).max(0.0);
    body
}

fn sanitize_collider(mut collider: Collider2D) -> Collider2D {
    collider.half_extents = finite_vec2(collider.half_extents).abs().max(Vec2::splat(0.0001));
    collider.radius = finite_or(collider.radius, 0.5).abs().max(0.0001);
    collider.offset = finite_vec2(collider.offset);
    collider.rotation = finite_or(collider.rotation, 0.0);
    collider.friction = finite_or(collider.friction, 0.0).max(0.0);
    collider.restitution = finite_or(collider.restitution, 0.0).clamp(0.0, 1.0);
    collider
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

fn finite_vec2(value: Vec2) -> Vec2 {
    Vec2::new(
        if value.x.is_finite() { value.x } else { 0.0 },
        if value.y.is_finite() { value.y } else { 0.0 },
    )
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_body(entity: u64, position: Vec2) -> BodySnapshot2D {
        BodySnapshot2D {
            entity: Entity(entity),
            body: RigidBody2D::dynamic(),
            collider: Collider2D::circle(0.5),
            position,
            rotation: 0.0,
            scale: Vec2::ONE,
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            has_body_component: true,
            has_velocity_component: true,
        }
    }

    #[test]
    fn broadphase_deduplicates_pairs_across_cells() {
        let bodies = vec![test_body(1, Vec2::ZERO), test_body(2, Vec2::new(0.2, 0.0))];
        let pairs = broadphase_pairs(&bodies, 0.25);
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn collision_masks_must_accept_each_other() {
        let mut a = Collider2D::default();
        let mut b = Collider2D::default();
        a.collision_layer = 1;
        a.collision_mask = 2;
        b.collision_layer = 2;
        b.collision_mask = 1;
        assert!(layers_interact(&a, &b));
        b.collision_mask = 4;
        assert!(!layers_interact(&a, &b));
    }
}
