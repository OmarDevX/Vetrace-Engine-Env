use crate::components::components::{GlobalTransform, Parent, Transform};
use crate::ecs::Entity;
use crate::{engine::engine::Engine, Behaviour};
use ahash::HashMap;
use glam::{Quat, Vec3};

pub struct HierarchySystem;

impl Default for HierarchySystem {
    fn default() -> Self {
        Self
    }
}

pub fn update_global_transforms(world: &mut crate::ecs::World) {
    fn calc_global(
        entity: Entity,
        world: &crate::ecs::World,
        cache: &mut HashMap<Entity, GlobalTransform>,
    ) -> GlobalTransform {
        if let Some(g) = cache.get(&entity).copied() {
            return g;
        }

        let local = world.get::<Transform>(entity).cloned().unwrap_or_default();
        let result = if let Some(parent) = world.get::<Parent>(entity) {
            let parent_global = calc_global(parent.entity, world, cache);
            let parent_q = Quat::from_xyzw(
                parent_global.orientation[0],
                parent_global.orientation[1],
                parent_global.orientation[2],
                parent_global.orientation[3],
            );
            let child_q = Quat::from_xyzw(
                local.orientation[0],
                local.orientation[1],
                local.orientation[2],
                local.orientation[3],
            );
            let global_q = (parent_q * child_q).normalize();
            let global_pos =
                parent_q * Vec3::from(local.position) + Vec3::from(parent_global.position);
            GlobalTransform {
                position: global_pos.into(),
                orientation: [global_q.x, global_q.y, global_q.z, global_q.w],
                size: local.size,
            }
        } else {
            GlobalTransform {
                position: local.position,
                orientation: local.orientation,
                size: local.size,
            }
        };

        cache.insert(entity, result);
        result
    }

    let entities: Vec<Entity> = world.entities().to_vec();
    let mut cache: HashMap<Entity, GlobalTransform> = HashMap::default();
    for e in entities {
        let g = calc_global(e, world, &mut cache);
        if let Some(existing) = world.get_mut::<GlobalTransform>(e) {
            *existing = g;
        } else {
            world.insert(e, g);
        }
    }
}

impl Behaviour for HierarchySystem {
    fn update(&mut self, engine: &mut Engine, _delta: f32) {
        update_global_transforms(&mut engine.world);
    }
}
