use glam::{Quat, Vec3};
use rapier3d::math::{Point, Vector};
use rapier3d::prelude::{QueryFilter, Ray};
use vetrace_core::backends::RaycastHit;
use vetrace_core::{Engine, Entity, GlobalTransform, Transform};

use crate::components::{Collider, ColliderShape, Raycast};
use crate::state::PhysicsState;

const RAY_EPSILON: f32 = 0.0001;

pub(crate) fn update_raycast_components(engine: &mut Engine) {
    let raycasts: Vec<_> = engine.raw_world().query::<Raycast>()
        .into_iter()
        .map(|(entity, raycast)| (entity, raycast.origin, raycast.direction, raycast.max_distance))
        .collect();

    let hits: Vec<_> = engine
        .get_resource::<PhysicsState>()
        .map(|state| {
            raycasts
                .iter()
                .map(|(entity, origin, direction, max_distance)| {
                    let dir = direction.normalize_or_zero();
                    if dir.length_squared() == 0.0 {
                        return (*entity, None);
                    }
                    let ray = Ray::new(Point::new(origin.x, origin.y, origin.z), Vector::new(dir.x, dir.y, dir.z));
                    let hit = state.query_pipeline.cast_ray(
                        &state.bodies,
                        &state.colliders,
                        &ray,
                        *max_distance,
                        true,
                        QueryFilter::default(),
                    );
                    let hit = hit.map(|(collider, distance)| RaycastHit {
                        entity: state.collider_entities.get(&collider).copied(),
                        position: *origin + dir * distance,
                        distance,
                    });
                    (*entity, hit)
                })
                .collect()
        })
        .unwrap_or_default();

    for (entity, hit) in hits {
        if let Some(raycast) = engine.raw_world_mut().get_mut::<Raycast>(entity) {
            raycast.hit = hit;
        }
    }
}

/// Casts a ray against authored `Collider` components directly from ECS state.
///
/// This is useful for gameplay queries that must be correct immediately after a
/// scene/map is spawned and should not depend on Rapier's next sync step.  It
/// still uses the shared physics collider definitions instead of renderer
/// bounds or game-specific wall checks.  `predicate` decides which collider
/// entities are eligible; callers can exclude the shooter, players, sensors, or
/// non-solid gameplay markers.
pub fn raycast_colliders(
    engine: &Engine,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    mut predicate: impl FnMut(Entity) -> bool,
) -> Option<RaycastHit> {
    let dir = direction.normalize_or_zero();
    if dir.length_squared() == 0.0 || max_distance <= 0.0 || !max_distance.is_finite() {
        return None;
    }

    let mut best: Option<RaycastHit> = None;
    for (entity, collider) in engine.raw_world().query::<Collider>() {
        if !predicate(entity) {
            continue;
        }
        let Some((translation, rotation, scale)) = collider_world_pose(engine, entity) else {
            continue;
        };
        let Some(distance) = raycast_single_collider(origin, dir, max_distance, translation, rotation, scale, collider) else {
            continue;
        };
        if distance < RAY_EPSILON {
            continue;
        }
        match best {
            Some(hit) if distance >= hit.distance => {}
            _ => {
                best = Some(RaycastHit {
                    entity: Some(entity),
                    position: origin + dir * distance,
                    distance,
                });
            }
        }
    }
    best
}

fn collider_world_pose(engine: &Engine, entity: Entity) -> Option<(Vec3, Quat, Vec3)> {
    if let Some(global) = engine.raw_world().get::<GlobalTransform>(entity) {
        return Some((global.translation, global.rotation.normalize(), global.scale));
    }
    let transform = engine.raw_world().get::<Transform>(entity)?;
    Some((transform.translation, transform.rotation.normalize(), transform.scale))
}

fn raycast_single_collider(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
    collider: &Collider,
) -> Option<f32> {
    let safe_scale = finite_vec3_or_one(scale).abs().max(Vec3::splat(0.001));
    let half_extents = (collider.half_extents.abs() * safe_scale).max(Vec3::splat(0.001));
    let center = translation + rotation * (collider.offset * safe_scale);
    let local_origin = rotation.inverse() * (origin - center);
    let local_dir = rotation.inverse() * dir;

    match collider.shape {
        ColliderShape::Cube => ray_aabb(local_origin, local_dir, half_extents, max_distance),
        ColliderShape::Sphere => ray_ellipsoid(local_origin, local_dir, half_extents, max_distance),
        ColliderShape::Capsule => ray_capsule_y(local_origin, local_dir, half_extents, max_distance)
            .or_else(|| ray_ellipsoid(local_origin, local_dir, half_extents, max_distance)),
    }
}

fn ray_aabb(origin: Vec3, dir: Vec3, half_extents: Vec3, max_distance: f32) -> Option<f32> {
    let mut t_min = 0.0_f32;
    let mut t_max = max_distance;

    for axis in 0..3 {
        let o = origin[axis];
        let d = dir[axis];
        let min = -half_extents[axis];
        let max = half_extents[axis];

        if d.abs() < RAY_EPSILON {
            if o < min || o > max {
                return None;
            }
            continue;
        }

        let inv = 1.0 / d;
        let mut t1 = (min - o) * inv;
        let mut t2 = (max - o) * inv;
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        t_min = t_min.max(t1);
        t_max = t_max.min(t2);
        if t_min > t_max {
            return None;
        }
    }

    if (0.0..=max_distance).contains(&t_min) {
        Some(t_min)
    } else if (0.0..=max_distance).contains(&t_max) {
        Some(t_max)
    } else {
        None
    }
}

fn ray_ellipsoid(origin: Vec3, dir: Vec3, radii: Vec3, max_distance: f32) -> Option<f32> {
    let p = origin / radii;
    let d = dir / radii;
    let a = d.dot(d);
    if a <= RAY_EPSILON {
        return None;
    }
    let b = 2.0 * p.dot(d);
    let c = p.dot(p) - 1.0;
    ray_quadratic_nearest(a, b, c, max_distance)
}

fn ray_capsule_y(origin: Vec3, dir: Vec3, half_extents: Vec3, max_distance: f32) -> Option<f32> {
    // Capsule colliders are authored as Y-up capsules. For non-uniform X/Z
    // scale, use the larger horizontal radius so the query stays conservative
    // and never lets shots pass through visible solid capsule geometry.
    let radius = half_extents.x.max(half_extents.z).max(0.001);
    let segment_half = (half_extents.y - radius).max(0.0);
    let mut best: Option<f32> = None;

    // Infinite cylinder body, clamped to the capsule segment.
    let a = dir.x * dir.x + dir.z * dir.z;
    if a > RAY_EPSILON {
        let b = 2.0 * (origin.x * dir.x + origin.z * dir.z);
        let c = origin.x * origin.x + origin.z * origin.z - radius * radius;
        for t in ray_quadratic_roots(a, b, c) {
            if !(0.0..=max_distance).contains(&t) {
                continue;
            }
            let y = origin.y + dir.y * t;
            if (-segment_half..=segment_half).contains(&y) {
                best = Some(best.map_or(t, |old| old.min(t)));
            }
        }
    }

    for cap_y in [-segment_half, segment_half] {
        let sphere_origin = origin - Vec3::Y * cap_y;
        if let Some(t) = ray_sphere(sphere_origin, dir, radius, max_distance) {
            best = Some(best.map_or(t, |old| old.min(t)));
        }
    }

    best
}

fn ray_sphere(origin: Vec3, dir: Vec3, radius: f32, max_distance: f32) -> Option<f32> {
    let a = dir.dot(dir);
    let b = 2.0 * origin.dot(dir);
    let c = origin.dot(origin) - radius * radius;
    ray_quadratic_nearest(a, b, c, max_distance)
}

fn ray_quadratic_nearest(a: f32, b: f32, c: f32, max_distance: f32) -> Option<f32> {
    ray_quadratic_roots(a, b, c)
        .into_iter()
        .filter(|t| (0.0..=max_distance).contains(t))
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn ray_quadratic_roots(a: f32, b: f32, c: f32) -> [f32; 2] {
    if a.abs() <= RAY_EPSILON {
        return [f32::INFINITY, f32::INFINITY];
    }
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 {
        return [f32::INFINITY, f32::INFINITY];
    }
    let sqrt_d = discriminant.sqrt();
    let inv = 0.5 / a;
    [(-b - sqrt_d) * inv, (-b + sqrt_d) * inv]
}

fn finite_vec3_or_one(value: Vec3) -> Vec3 {
    Vec3::new(finite_or_one(value.x), finite_or_one(value.y), finite_or_one(value.z))
}

fn finite_or_one(value: f32) -> f32 {
    if value.is_finite() { value } else { 1.0 }
}

