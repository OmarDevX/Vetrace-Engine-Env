use super::*;

/// Create a simple renderable cube for editor smoke tests.
pub fn spawn_editor_test_cube(engine: &mut Engine, name: impl Into<String>, translation: Vec3) -> Entity {
    let entity = engine.spawn_actor(name).build().entity();
    engine.raw_world_mut().insert(entity, Transform { translation, rotation: Quat::IDENTITY, scale: Vec3::ONE });
    engine.raw_world_mut().insert(entity, Shape { primitive: PrimitiveShape::Cube, size: Vec3::ONE });
    engine.raw_world_mut().insert(entity, Renderable { mesh: None, material: None, visible: true });
    engine.raw_world_mut().insert(entity, Material { base_color: Vec3::new(0.25, 0.55, 1.0), ..Material::default() });
    entity
}

/// Spawn a small translucent editor HUD block using the active renderer overlay
/// path.  Text rendering belongs to a future egui/text plugin; this is only a
/// smoke-test visible marker that the editor crate is active.
pub fn spawn_editor_overlay_marker(engine: &mut Engine) -> Entity {
    let entity = engine.spawn_actor("Editor Overlay Marker").build().entity();
    engine.raw_world_mut().insert(entity, EditorOnly);
    engine.raw_world_mut().insert(entity, ScreenSpaceRect {
        anchor: Vec2::new(0.0, 0.0),
        offset_px: Vec2::new(14.0, 14.0),
        size_px: Vec2::new(180.0, 36.0),
        z_order: 10,
    });
    engine.raw_world_mut().insert(entity, Material {
        base_color: Vec3::new(0.08, 0.02, 0.12),
        emissive: Vec3::new(0.08, 0.02, 0.12),
        alpha: 0.75,
        ..Material::default()
    });
    entity
}

/// Build a view matrix for tools that want to draw gizmos externally.
pub fn editor_view_matrix(camera: &Camera) -> Mat4 {
    Mat4::look_at_rh(camera.position, camera.target, camera.up)
}
