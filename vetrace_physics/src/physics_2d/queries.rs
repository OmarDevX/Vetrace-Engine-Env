use glam::Vec2;
use vetrace_core::{Engine, Entity};

use super::geometry::{collide, point_inside, raycast_shape, WorldShape2D};
use super::transforms::current_physics_transform_2d;
use super::Collider2D;

#[derive(Clone, Copy, Debug)]
pub struct Physics2dQueryFilter {
    pub layer_mask: u32,
    pub include_sensors: bool,
    pub exclude: Option<Entity>,
}

impl Default for Physics2dQueryFilter {
    fn default() -> Self {
        Self { layer_mask: u32::MAX, include_sensors: true, exclude: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaycastHit2D {
    pub entity: Entity,
    pub position: Vec2,
    pub normal: Vec2,
    pub distance: f32,
}

pub fn raycast_2d(
    engine: &Engine,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
    filter: Physics2dQueryFilter,
) -> Option<RaycastHit2D> {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO || !max_distance.is_finite() || max_distance < 0.0 { return None; }
    let mut best: Option<RaycastHit2D> = None;
    for (entity, collider, shape) in query_shapes(engine, filter) {
        let Some((distance, normal)) = raycast_shape(shape, origin, direction, max_distance) else { continue; };
        if best.is_some_and(|hit| hit.distance <= distance) { continue; }
        best = Some(RaycastHit2D {
            entity,
            position: origin + direction * distance,
            normal,
            distance,
        });
        let _ = collider;
    }
    best
}

pub fn point_query_2d(
    engine: &Engine,
    point: Vec2,
    filter: Physics2dQueryFilter,
) -> Vec<Entity> {
    query_shapes(engine, filter)
        .filter_map(|(entity, _, shape)| point_inside(shape, point).then_some(entity))
        .collect()
}

pub fn overlap_circle_2d(
    engine: &Engine,
    center: Vec2,
    radius: f32,
    filter: Physics2dQueryFilter,
) -> Vec<Entity> {
    let query = WorldShape2D::Circle { center, radius: radius.abs().max(0.0001) };
    query_shapes(engine, filter)
        .filter_map(|(entity, _, shape)| collide(query, shape).is_some().then_some(entity))
        .collect()
}

pub fn overlap_box_2d(
    engine: &Engine,
    center: Vec2,
    half_extents: Vec2,
    rotation: f32,
    filter: Physics2dQueryFilter,
) -> Vec<Entity> {
    let query = WorldShape2D::Box {
        center,
        half_extents: half_extents.abs().max(Vec2::splat(0.0001)),
        rotation,
    };
    query_shapes(engine, filter)
        .filter_map(|(entity, _, shape)| collide(query, shape).is_some().then_some(entity))
        .collect()
}

fn query_shapes(
    engine: &Engine,
    filter: Physics2dQueryFilter,
) -> impl Iterator<Item = (Entity, Collider2D, WorldShape2D)> + '_ {
    engine
        .raw_world()
        .query::<Collider2D>()
        .into_iter()
        .filter_map(move |(entity, collider)| {
            if !collider.enabled
                || (!filter.include_sensors && collider.sensor)
                || filter.exclude == Some(entity)
                || collider.collision_layer & filter.layer_mask == 0
            {
                return None;
            }
            if engine.raw_world().get::<vetrace_core::Transform>(entity).is_none() { return None; }
            let transform = current_physics_transform_2d(engine, entity);
            let shape = WorldShape2D::from_collider(
                transform.position,
                transform.rotation,
                transform.scale,
                collider,
            );
            Some((entity, (*collider).clone(), shape))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use vetrace_core::Transform;

    #[test]
    fn raycast_returns_nearest_entity() {
        let mut engine = Engine::new();
        let far = engine
            .spawn_actor("far")
            .with(Transform { translation: Vec3::new(5.0, 0.0, 0.0), ..Transform::default() })
            .with(Collider2D::circle(0.5))
            .build()
            .entity();
        let near = engine
            .spawn_actor("near")
            .with(Transform { translation: Vec3::new(2.0, 0.0, 0.0), ..Transform::default() })
            .with(Collider2D::circle(0.5))
            .build()
            .entity();
        let hit = raycast_2d(&engine, Vec2::ZERO, Vec2::X, 10.0, Physics2dQueryFilter::default()).unwrap();
        assert_eq!(hit.entity, near);
        assert_ne!(hit.entity, far);
    }
}
