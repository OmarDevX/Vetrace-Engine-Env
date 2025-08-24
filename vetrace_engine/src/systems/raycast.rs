use glam::{Vec3, Quat};

use crate::{
    Behaviour,
    engine::engine::Engine,
    components::components::Raycast,
    math::array_to_vec3,
};

pub struct RaycastSystem;

impl RaycastSystem {
    fn sphere_intersect(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
        let oc = origin - center;
        let b = oc.dot(dir);
        let c = oc.length_squared() - radius * radius;
        let disc = b * b - c;
        if disc < 0.0 {
            return None;
        }
        let s = disc.sqrt();
        let t1 = -b - s;
        if t1 > 0.0 {
            Some(t1)
        } else {
            let t2 = -b + s;
            if t2 > 0.0 { Some(t2) } else { None }
        }
    }

    fn cube_intersect(origin: Vec3, dir: Vec3, pos: Vec3, size: Vec3, orient: Quat) -> Option<f32> {
        let inv_q = orient.conjugate();
        let local_origin = inv_q * (origin - pos);
        let local_dir = inv_q * dir;
        let inv = Vec3::new(1.0 / local_dir.x, 1.0 / local_dir.y, 1.0 / local_dir.z);
        let min_b = -size * 0.5;
        let max_b = size * 0.5;
        let t0 = (min_b - local_origin) * inv;
        let t1 = (max_b - local_origin) * inv;
        let tmin = f32::max(f32::max(t0.x.min(t1.x), t0.y.min(t1.y)), t0.z.min(t1.z));
        let tmax = f32::min(f32::min(t0.x.max(t1.x), t0.y.max(t1.y)), t0.z.max(t1.z));
        if tmax >= tmin.max(0.0) {
            if tmin > 0.0 {
                Some(tmin)
            } else if tmax > 0.0 {
                Some(tmax)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Behaviour for RaycastSystem {
    fn update(&mut self, engine: &mut Engine, _dt: f32) {
        let objects = &engine.scene.objects;
        for (_e, ray) in engine.world.query_mut::<Raycast>() {
            let origin = Vec3::from(ray.origin);
            let dir = Vec3::from(ray.direction).normalize();
            let mut best = ray.max_distance;
            let mut hit_entity = crate::ecs::Entity(0);
            for (i, obj) in objects.iter().enumerate() {
                if let Some(ent) = engine.core.find_entity_by_object_id(i as u32) {
                    if ent == ray.ignore_entity {
                        continue;
                    }
                    let pos = array_to_vec3(obj.position);
                    let orient = Quat::from_xyzw(
                        obj.orientation[0],
                        obj.orientation[1],
                        obj.orientation[2],
                        obj.orientation[3],
                    );
                    let size = array_to_vec3(obj.size) * array_to_vec3(obj.scale);
                    let t = if obj.is_cube {
                        Self::cube_intersect(origin, dir, pos, size, orient)
                    } else {
                        let s = obj.scale[0].max(obj.scale[1]).max(obj.scale[2]);
                        Self::sphere_intersect(origin, dir, pos, obj.radius * s)
                    };
                    if let Some(dist) = t {
                        if dist < best && dist <= ray.max_distance {
                            best = dist;
                            hit_entity = ent;
                        }
                    }
                }
            }
            ray.hit_distance = best;
            let hit = origin + dir * best;
            ray.hit_position = [hit.x, hit.y, hit.z];
            ray.hit_entity = hit_entity;
        }
    }
}
