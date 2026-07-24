use glam::{EulerRot, Quat, Vec2};
use vetrace_core::{Engine, Entity, Parent, Transform};

use super::geometry::rotate;

#[derive(Clone, Copy, Debug)]
pub(crate) struct PhysicsTransform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Default for PhysicsTransform2D {
    fn default() -> Self {
        Self { position: Vec2::ZERO, rotation: 0.0, scale: Vec2::ONE }
    }
}

pub(crate) fn current_physics_transform_2d(engine: &Engine, entity: Entity) -> PhysicsTransform2D {
    current_recursive(engine, entity, &mut Vec::new())
}

fn current_recursive(
    engine: &Engine,
    entity: Entity,
    stack: &mut Vec<Entity>,
) -> PhysicsTransform2D {
    if stack.contains(&entity) {
        return local_transform(engine, entity);
    }
    stack.push(entity);
    let local = local_transform(engine, entity);
    let global = if let Some(parent) = engine.raw_world().get::<Parent>(entity) {
        let parent = current_recursive(engine, parent.0, stack);
        PhysicsTransform2D {
            position: parent.position + rotate(local.position * parent.scale, parent.rotation),
            rotation: parent.rotation + local.rotation,
            scale: parent.scale * local.scale,
        }
    } else {
        local
    };
    stack.pop();
    global
}

pub(crate) fn write_world_pose_2d(
    engine: &mut Engine,
    entity: Entity,
    world_position: Vec2,
    world_rotation: f32,
) {
    let parent_global = engine
        .raw_world()
        .get::<Parent>(entity)
        .map(|parent| current_physics_transform_2d(engine, parent.0));
    if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
        if let Some(parent) = parent_global {
            let safe_scale = parent.scale.abs().max(Vec2::splat(0.0001));
            let local_position = rotate(world_position - parent.position, -parent.rotation) / safe_scale;
            transform.translation.x = local_position.x;
            transform.translation.y = local_position.y;
            transform.rotation = Quat::from_rotation_z(world_rotation - parent.rotation);
        } else {
            transform.translation.x = world_position.x;
            transform.translation.y = world_position.y;
            transform.rotation = Quat::from_rotation_z(world_rotation);
        }
    }
}

fn local_transform(engine: &Engine, entity: Entity) -> PhysicsTransform2D {
    let Some(transform) = engine.raw_world().get::<Transform>(entity) else {
        return PhysicsTransform2D::default();
    };
    let (_, _, rotation) = transform.rotation.to_euler(EulerRot::XYZ);
    PhysicsTransform2D {
        position: finite_vec2(transform.translation.truncate()),
        rotation: if rotation.is_finite() { rotation } else { 0.0 },
        scale: finite_vec2(transform.scale.truncate()).abs().max(Vec2::splat(0.0001)),
    }
}

fn finite_vec2(value: Vec2) -> Vec2 {
    Vec2::new(
        if value.x.is_finite() { value.x } else { 0.0 },
        if value.y.is_finite() { value.y } else { 0.0 },
    )
}
