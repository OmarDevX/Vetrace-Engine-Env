use super::*;

pub(crate) fn draw_overlay_rect(canvas: &mut Canvas<Window>, overlay: &RenderOverlayRect, width: f32, height: f32) {
    let center = Vec2::new(overlay.rect.anchor.x * width, overlay.rect.anchor.y * height) + overlay.rect.offset_px;
    let size = overlay.rect.size_px.max(Vec2::splat(1.0));
    let c = (overlay.material.base_color + overlay.material.emissive).clamp(Vec3::ZERO, Vec3::ONE);
    canvas.set_draw_color(Color::RGBA(
        (c.x * 255.0) as u8,
        (c.y * 255.0) as u8,
        (c.z * 255.0) as u8,
        (overlay.material.alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ));
    let rect = Rect::new(
        (center.x - size.x * 0.5).round() as i32,
        (center.y - size.y * 0.5).round() as i32,
        size.x.round().max(1.0) as u32,
        size.y.round().max(1.0) as u32,
    );
    let _ = canvas.fill_rect(rect);
}

pub(crate) fn draw_billboard_sprite(canvas: &mut Canvas<Window>, sprite: &RenderSprite, frame: &RenderFrame, width: f32, height: f32) {
    let Some(screen) = project_to_screen(sprite.transform.translation, &frame.camera, width, height) else { return; };
    let distance = frame.camera.position.distance(sprite.transform.translation).max(0.1);
    let scale = (height / distance).clamp(4.0, 256.0);
    let size_x = sprite.sprite.size.x.max(0.05) * scale;
    let size_y = sprite.sprite.size.y.max(0.05) * scale;
    let color = color_from_material(&sprite.material, Vec3::Y, frame, 1.0);
    canvas.set_draw_color(color);
    let rect = Rect::new(
        (screen.x - size_x * 0.5) as i32,
        (screen.y - size_y * 0.5) as i32,
        size_x.max(1.0) as u32,
        size_y.max(1.0) as u32,
    );
    let _ = canvas.fill_rect(rect);
    canvas.set_draw_color(darken(color, 0.25));
    let _ = canvas.draw_rect(rect);
}

trait DefaultShapeExt {
    fn unwrap_or_default_shape(self) -> Shape;
}
