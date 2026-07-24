use super::*;

pub(super) fn extract_entity_2d(
    engine: &Engine,
    entity: Entity,
    transform: &GlobalTransform,
    scene: &mut SceneExtraction,
) {
    let Some(sprite) = engine.raw_world().get::<Sprite2D>(entity) else {
        return;
    };
    let canvas = engine
        .raw_world()
        .get::<CanvasItem2D>(entity)
        .cloned()
        .unwrap_or_default();
    if !canvas.visible
        || !sprite.size.is_finite()
        || sprite.size.x.abs() <= f32::EPSILON
        || sprite.size.y.abs() <= f32::EPSILON
        || !transform.translation.is_finite()
        || !transform.scale.is_finite()
    {
        return;
    }
    scene.sprites_2d.push(RenderSprite2D {
        entity,
        transform: transform.clone(),
        canvas,
        sprite: sprite.clone(),
    });
}
