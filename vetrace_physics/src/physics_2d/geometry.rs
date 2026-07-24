use glam::Vec2;

use super::{Collider2D, ColliderShape2D};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Aabb2D {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb2D {
    pub fn overlaps(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum WorldShape2D {
    Circle { center: Vec2, radius: f32 },
    Box { center: Vec2, half_extents: Vec2, rotation: f32 },
}

impl WorldShape2D {
    pub fn from_collider(position: Vec2, rotation: f32, scale: Vec2, collider: &Collider2D) -> Self {
        let safe_scale = scale.abs().max(Vec2::splat(0.0001));
        let center = position + rotate(collider.offset * safe_scale, rotation);
        match collider.shape {
            ColliderShape2D::Circle => Self::Circle {
                center,
                radius: (collider.radius.abs() * safe_scale.max_element()).max(0.0001),
            },
            ColliderShape2D::Box => Self::Box {
                center,
                half_extents: (collider.half_extents.abs() * safe_scale).max(Vec2::splat(0.0001)),
                rotation: rotation + collider.rotation,
            },
        }
    }

    pub fn center(self) -> Vec2 {
        match self {
            Self::Circle { center, .. } | Self::Box { center, .. } => center,
        }
    }

    pub fn minimum_extent(self) -> f32 {
        match self {
            Self::Circle { radius, .. } => radius,
            Self::Box { half_extents, .. } => half_extents.min_element(),
        }
        .max(0.0001)
    }

    pub fn aabb(self) -> Aabb2D {
        match self {
            Self::Circle { center, radius } => Aabb2D {
                min: center - Vec2::splat(radius),
                max: center + Vec2::splat(radius),
            },
            Self::Box { center, half_extents, rotation } => {
                let (axis_x, axis_y) = box_axes(rotation);
                let world_half = Vec2::new(
                    axis_x.x.abs() * half_extents.x + axis_y.x.abs() * half_extents.y,
                    axis_x.y.abs() * half_extents.x + axis_y.y.abs() * half_extents.y,
                );
                Aabb2D { min: center - world_half, max: center + world_half }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ContactManifold2D {
    /// Unit normal pointing from shape A toward shape B.
    pub normal: Vec2,
    pub point: Vec2,
    pub penetration: f32,
}

pub(crate) fn collide(a: WorldShape2D, b: WorldShape2D) -> Option<ContactManifold2D> {
    match (a, b) {
        (
            WorldShape2D::Circle { center: center_a, radius: radius_a },
            WorldShape2D::Circle { center: center_b, radius: radius_b },
        ) => collide_circle_circle(center_a, radius_a, center_b, radius_b),
        (
            WorldShape2D::Box { center: center_a, half_extents: half_a, rotation: rotation_a },
            WorldShape2D::Box { center: center_b, half_extents: half_b, rotation: rotation_b },
        ) => collide_box_box(center_a, half_a, rotation_a, center_b, half_b, rotation_b),
        (
            WorldShape2D::Circle { center, radius },
            WorldShape2D::Box { center: box_center, half_extents, rotation },
        ) => collide_circle_box(center, radius, box_center, half_extents, rotation),
        (
            WorldShape2D::Box { center: box_center, half_extents, rotation },
            WorldShape2D::Circle { center, radius },
        ) => collide_circle_box(center, radius, box_center, half_extents, rotation).map(|contact| {
            ContactManifold2D { normal: -contact.normal, ..contact }
        }),
    }
}

fn collide_circle_circle(
    center_a: Vec2,
    radius_a: f32,
    center_b: Vec2,
    radius_b: f32,
) -> Option<ContactManifold2D> {
    let delta = center_b - center_a;
    let radius_sum = radius_a + radius_b;
    let distance_squared = delta.length_squared();
    if distance_squared > radius_sum * radius_sum { return None; }
    let distance = distance_squared.sqrt();
    let normal = if distance > 0.000001 { delta / distance } else { Vec2::X };
    let penetration = (radius_sum - distance).max(0.0);
    let point = center_a + normal * (radius_a - penetration * 0.5);
    Some(ContactManifold2D { normal, point, penetration })
}

fn collide_box_box(
    center_a: Vec2,
    half_a: Vec2,
    rotation_a: f32,
    center_b: Vec2,
    half_b: Vec2,
    rotation_b: f32,
) -> Option<ContactManifold2D> {
    let (a_x, a_y) = box_axes(rotation_a);
    let (b_x, b_y) = box_axes(rotation_b);
    let delta = center_b - center_a;
    let mut best_axis = Vec2::X;
    let mut best_overlap = f32::MAX;

    for axis in [a_x, a_y, b_x, b_y] {
        let radius_a = half_a.x * axis.dot(a_x).abs() + half_a.y * axis.dot(a_y).abs();
        let radius_b = half_b.x * axis.dot(b_x).abs() + half_b.y * axis.dot(b_y).abs();
        let distance = delta.dot(axis);
        let overlap = radius_a + radius_b - distance.abs();
        if overlap < 0.0 { return None; }
        if overlap < best_overlap {
            best_overlap = overlap;
            best_axis = if distance >= 0.0 { axis } else { -axis };
        }
    }

    let point_a = support_box(center_a, half_a, a_x, a_y, best_axis);
    let point_b = support_box(center_b, half_b, b_x, b_y, -best_axis);
    Some(ContactManifold2D {
        normal: best_axis.normalize_or_zero(),
        point: (point_a + point_b) * 0.5,
        penetration: best_overlap.max(0.0),
    })
}

fn collide_circle_box(
    circle_center: Vec2,
    circle_radius: f32,
    box_center: Vec2,
    half_extents: Vec2,
    box_rotation: f32,
) -> Option<ContactManifold2D> {
    let local_circle = rotate(circle_center - box_center, -box_rotation);
    let closest = local_circle.clamp(-half_extents, half_extents);
    let from_box_to_circle = local_circle - closest;
    let distance_squared = from_box_to_circle.length_squared();

    if distance_squared > 0.000001 {
        if distance_squared > circle_radius * circle_radius { return None; }
        let distance = distance_squared.sqrt();
        let box_to_circle_local = from_box_to_circle / distance;
        let circle_to_box = -rotate(box_to_circle_local, box_rotation);
        return Some(ContactManifold2D {
            normal: circle_to_box,
            point: box_center + rotate(closest, box_rotation),
            penetration: (circle_radius - distance).max(0.0),
        });
    }

    // Circle center is inside the box. Push it toward the nearest face.
    let distance_to_x = half_extents.x - local_circle.x.abs();
    let distance_to_y = half_extents.y - local_circle.y.abs();
    let outward_local = if distance_to_x < distance_to_y {
        Vec2::new(nonzero_sign(local_circle.x), 0.0)
    } else {
        Vec2::new(0.0, nonzero_sign(local_circle.y))
    };
    let outward = rotate(outward_local, box_rotation);
    let face_distance = distance_to_x.min(distance_to_y).max(0.0);
    Some(ContactManifold2D {
        // Normal is from circle toward box, opposite the direction used to eject it.
        normal: -outward,
        point: circle_center + outward * face_distance,
        penetration: circle_radius + face_distance,
    })
}

pub(crate) fn point_inside(shape: WorldShape2D, point: Vec2) -> bool {
    match shape {
        WorldShape2D::Circle { center, radius } => center.distance_squared(point) <= radius * radius,
        WorldShape2D::Box { center, half_extents, rotation } => {
            let local = rotate(point - center, -rotation);
            local.x.abs() <= half_extents.x && local.y.abs() <= half_extents.y
        }
    }
}

pub(crate) fn raycast_shape(
    shape: WorldShape2D,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> Option<(f32, Vec2)> {
    let direction = direction.normalize_or_zero();
    if direction == Vec2::ZERO || max_distance < 0.0 { return None; }
    match shape {
        WorldShape2D::Circle { center, radius } => raycast_circle(center, radius, origin, direction, max_distance),
        WorldShape2D::Box { center, half_extents, rotation } => {
            raycast_box(center, half_extents, rotation, origin, direction, max_distance)
        }
    }
}

fn raycast_circle(
    center: Vec2,
    radius: f32,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> Option<(f32, Vec2)> {
    let offset = origin - center;
    let b = offset.dot(direction);
    let c = offset.length_squared() - radius * radius;
    if c <= 0.0 {
        let normal = (origin - center).normalize_or_zero();
        return Some((0.0, if normal == Vec2::ZERO { -direction } else { normal }));
    }
    let discriminant = b * b - c;
    if discriminant < 0.0 { return None; }
    let distance = -b - discriminant.sqrt();
    if distance < 0.0 || distance > max_distance { return None; }
    let point = origin + direction * distance;
    Some((distance, (point - center).normalize_or_zero()))
}

fn raycast_box(
    center: Vec2,
    half_extents: Vec2,
    rotation: f32,
    origin: Vec2,
    direction: Vec2,
    max_distance: f32,
) -> Option<(f32, Vec2)> {
    let local_origin = rotate(origin - center, -rotation);
    let local_direction = rotate(direction, -rotation);
    let mut t_min = 0.0_f32;
    let mut t_max = max_distance;
    let mut hit_normal = Vec2::ZERO;

    for (origin_axis, direction_axis, half, negative_normal, positive_normal) in [
        (local_origin.x, local_direction.x, half_extents.x, -Vec2::X, Vec2::X),
        (local_origin.y, local_direction.y, half_extents.y, -Vec2::Y, Vec2::Y),
    ] {
        if direction_axis.abs() < 0.000001 {
            if origin_axis < -half || origin_axis > half { return None; }
            continue;
        }
        let inv = 1.0 / direction_axis;
        let mut near = (-half - origin_axis) * inv;
        let mut far = (half - origin_axis) * inv;
        let mut near_normal = negative_normal;
        if near > far {
            std::mem::swap(&mut near, &mut far);
            near_normal = positive_normal;
        }
        if near > t_min {
            t_min = near;
            hit_normal = near_normal;
        }
        t_max = t_max.min(far);
        if t_min > t_max { return None; }
    }

    if t_min < 0.0 || t_min > max_distance { return None; }
    if hit_normal == Vec2::ZERO { hit_normal = -local_direction.normalize_or_zero(); }
    Some((t_min, rotate(hit_normal, rotation)))
}

pub(crate) fn rotate(vector: Vec2, radians: f32) -> Vec2 {
    let (sin, cos) = radians.sin_cos();
    Vec2::new(vector.x * cos - vector.y * sin, vector.x * sin + vector.y * cos)
}

fn box_axes(rotation: f32) -> (Vec2, Vec2) {
    (rotate(Vec2::X, rotation), rotate(Vec2::Y, rotation))
}

fn support_box(center: Vec2, half: Vec2, axis_x: Vec2, axis_y: Vec2, direction: Vec2) -> Vec2 {
    center
        + axis_x * if direction.dot(axis_x) >= 0.0 { half.x } else { -half.x }
        + axis_y * if direction.dot(axis_y) >= 0.0 { half.y } else { -half.y }
}

fn nonzero_sign(value: f32) -> f32 {
    if value < 0.0 { -1.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_circle_contact_normal_points_from_a_to_b() {
        let contact = collide(
            WorldShape2D::Circle { center: Vec2::ZERO, radius: 1.0 },
            WorldShape2D::Circle { center: Vec2::new(1.5, 0.0), radius: 1.0 },
        )
        .unwrap();
        assert!((contact.normal - Vec2::X).length() < 0.0001);
        assert!((contact.penetration - 0.5).abs() < 0.0001);
    }

    #[test]
    fn rotated_boxes_overlap() {
        assert!(collide(
            WorldShape2D::Box { center: Vec2::ZERO, half_extents: Vec2::ONE, rotation: 0.5 },
            WorldShape2D::Box { center: Vec2::new(1.0, 0.0), half_extents: Vec2::ONE, rotation: -0.25 },
        )
        .is_some());
    }

    #[test]
    fn ray_hits_oriented_box() {
        let hit = raycast_shape(
            WorldShape2D::Box { center: Vec2::ZERO, half_extents: Vec2::ONE, rotation: 0.4 },
            Vec2::new(-5.0, 0.0),
            Vec2::X,
            10.0,
        );
        assert!(hit.is_some());
    }
}
