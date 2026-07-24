use super::*;

pub(crate) fn pick_entity_2d_from_mouse(
    engine: &Engine,
    mouse: (f32, f32),
) -> Option<(Entity, f32)> {
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let camera = engine.get_resource::<Camera2D>().cloned().unwrap_or_default();
    let world = camera.screen_to_world(
        Vec2::new(mouse.0, mouse.1),
        Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32),
    );
    let mut candidates = engine
        .raw_world()
        .entities()
        .filter_map(|entity| {
            if engine.raw_world().has::<EditorOnly>(entity) {
                return None;
            }
            let visible_sprite = engine
                .raw_world()
                .get::<Sprite2D>(entity)
                .and_then(|sprite| {
                    let canvas = engine
                        .raw_world()
                        .get::<CanvasItem2D>(entity)
                        .cloned()
                        .unwrap_or_default();
                    canvas.visible.then_some(canvas)
                });
            #[cfg(feature = "physics_2d")]
            let has_collider = engine
                .raw_world()
                .get::<Collider2D>(entity)
                .map(|collider| collider.enabled)
                .unwrap_or(false);
            #[cfg(not(feature = "physics_2d"))]
            let has_collider = false;
            if visible_sprite.is_none() && !has_collider {
                return None;
            }
            Some((entity, visible_sprite.unwrap_or_default()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        a.1.canvas_layer
            .cmp(&b.1.canvas_layer)
            .then_with(|| a.1.z_index.cmp(&b.1.z_index))
            .then_with(|| a.0.0.cmp(&b.0.0))
    });

    for (entity, _) in candidates.into_iter().rev() {
        if hit_sprite(engine, entity, world) || hit_collider(engine, entity, world) {
            return Some((entity, 0.0));
        }
    }
    None
}

fn hit_sprite(engine: &Engine, entity: Entity, world: Vec2) -> bool {
    let Some(sprite) = engine.raw_world().get::<Sprite2D>(entity) else { return false; };
    let canvas = engine
        .raw_world()
        .get::<CanvasItem2D>(entity)
        .cloned()
        .unwrap_or_default();
    if !canvas.visible { return false; }
    let transform = global_transform_for(engine, entity);
    let local3 = transform.rotation.conjugate()
        * Vec3::new(
            world.x - transform.translation.x,
            world.y - transform.translation.y,
            0.0,
        );
    let scale = transform.scale.truncate();
    if scale.x.abs() <= 1.0e-6 || scale.y.abs() <= 1.0e-6 {
        return false;
    }
    let local = local3.truncate() / scale;
    let min = -sprite.size * sprite.pivot;
    let max = min + sprite.size;
    local.x >= min.x.min(max.x)
        && local.x <= min.x.max(max.x)
        && local.y >= min.y.min(max.y)
        && local.y <= min.y.max(max.y)
}

#[cfg(feature = "physics_2d")]
fn hit_collider(engine: &Engine, entity: Entity, world: Vec2) -> bool {
    let Some(collider) = engine.raw_world().get::<Collider2D>(entity) else { return false; };
    if !collider.enabled { return false; }
    let transform = global_transform_for(engine, entity);
    let scale = transform.scale.truncate();
    if scale.x.abs() <= 1.0e-6 || scale.y.abs() <= 1.0e-6 {
        return false;
    }
    let local3 = transform.rotation.conjugate()
        * Vec3::new(
            world.x - transform.translation.x,
            world.y - transform.translation.y,
            0.0,
        );
    let mut local = local3.truncate() / scale - collider.offset;
    let (sin, cos) = (-collider.rotation).sin_cos();
    local = Vec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos);
    match collider.shape {
        ColliderShape2D::Circle => local.length_squared() <= collider.radius.abs().powi(2),
        ColliderShape2D::Box => {
            let half = collider.half_extents.abs();
            local.x.abs() <= half.x && local.y.abs() <= half.y
        }
    }
}

#[cfg(not(feature = "physics_2d"))]
fn hit_collider(_engine: &Engine, _entity: Entity, _world: Vec2) -> bool { false }
