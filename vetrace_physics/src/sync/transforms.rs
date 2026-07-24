use super::*;

#[derive(Clone, Debug)]
pub(super) struct PhysicsEntityTransform {
    pub(super) global: GlobalTransform,
}

impl PhysicsEntityTransform {
    pub(super) fn pose(&self) -> PhysicsPose {
        PhysicsPose {
            translation: self.global.translation,
            rotation: self.global.rotation.normalize(),
        }
    }

    pub(super) fn scale(&self) -> Vec3 {
        self.global.scale
    }
}

fn combine_global(parent: &GlobalTransform, local: &Transform) -> GlobalTransform {
    GlobalTransform {
        translation: parent.translation + parent.rotation * (local.translation * parent.scale),
        rotation: (parent.rotation * local.rotation).normalize(),
        scale: parent.scale * local.scale,
    }
}

pub(super) fn current_global_transform(
    engine: &Engine,
    entity: Entity,
    stack: &mut Vec<Entity>,
) -> GlobalTransform {
    if stack.contains(&entity) {
        return engine
            .raw_world()
            .get::<Transform>(entity)
            .map(GlobalTransform::from)
            .unwrap_or_default();
    }

    stack.push(entity);
    let local = engine
        .raw_world()
        .get::<Transform>(entity)
        .cloned()
        .unwrap_or_default();
    let global = if let Some(parent) = engine.raw_world().get::<Parent>(entity) {
        let parent_global = current_global_transform(engine, parent.0, stack);
        combine_global(&parent_global, &local)
    } else {
        GlobalTransform::from(&local)
    };
    stack.pop();
    global
}

pub(super) fn physics_entity_transform(
    engine: &Engine,
    entity: Entity,
) -> PhysicsEntityTransform {
    PhysicsEntityTransform {
        global: current_global_transform(engine, entity, &mut Vec::new()),
    }
}

pub(super) fn write_world_pose_to_local_transform(
    engine: &mut Engine,
    entity: Entity,
    pose: PhysicsPose,
) {
    let parent_global = engine
        .raw_world()
        .get::<Parent>(entity)
        .map(|parent| current_global_transform(engine, parent.0, &mut Vec::new()));

    if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
        if let Some(parent) = parent_global {
            let inv_parent_rotation = parent.rotation.normalize().inverse();
            let safe_parent_scale = parent.scale.abs().max(Vec3::splat(0.001));
            transform.translation =
                inv_parent_rotation * (pose.translation - parent.translation) / safe_parent_scale;
            transform.rotation = (inv_parent_rotation * pose.rotation).normalize();
        } else {
            transform.translation = pose.translation;
            transform.rotation = pose.rotation;
        }
    }
}
