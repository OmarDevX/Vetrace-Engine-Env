use super::*;

pub(crate) fn draw_canvas_2d(
    canvas: &mut Canvas<Window>,
    frame: &RenderFrame,
    assets: Option<&RenderAssets>,
    width: f32,
    height: f32,
) {
    if frame.sprites_2d.is_empty() {
        return;
    }

    let viewport = Vec2::new(width.max(1.0), height.max(1.0));
    for render_sprite in &frame.sprites_2d {
        draw_canvas_sprite_2d(canvas, render_sprite, frame, assets, viewport);
    }
}

fn draw_canvas_sprite_2d(
    canvas: &mut Canvas<Window>,
    render_sprite: &crate::backend::RenderSprite2D,
    frame: &RenderFrame,
    assets: Option<&RenderAssets>,
    viewport: Vec2,
) {
    let sprite = &render_sprite.sprite;
    let transform = &render_sprite.transform;
    let camera = &frame.camera_2d;
    let pixels_per_unit = camera.pixels_per_world_unit();
    let world_pivot = transform.translation.truncate();
    let pivot_screen = camera.world_to_screen(world_pivot, viewport);
    let pixel_size = (sprite.size * transform.scale.truncate()).abs() * pixels_per_unit;
    if pixel_size.x <= 0.0 || pixel_size.y <= 0.0 {
        return;
    }

    let destination = Rect::new(
        (pivot_screen.x - pixel_size.x * sprite.pivot.x).round() as i32,
        (pivot_screen.y - pixel_size.y * (1.0 - sprite.pivot.y)).round() as i32,
        pixel_size.x.round().max(1.0) as u32,
        pixel_size.y.round().max(1.0) as u32,
    );
    let rotation_z = transform.rotation.to_euler(glam::EulerRot::XYZ).2;
    let angle_degrees = -((rotation_z - camera.rotation).to_degrees() as f64);
    let center = Point::new(
        (pixel_size.x * sprite.pivot.x).round() as i32,
        (pixel_size.y * (1.0 - sprite.pivot.y)).round() as i32,
    );

    let tint = sprite.tint.clamp(glam::Vec4::ZERO, glam::Vec4::ONE);
    let color = Color::RGBA(
        (tint.x * 255.0) as u8,
        (tint.y * 255.0) as u8,
        (tint.z * 255.0) as u8,
        (tint.w * 255.0) as u8,
    );

    let Some(texture_asset) = sprite
        .texture
        .and_then(|handle| assets.and_then(|assets| assets.textures.get(&handle.0)))
    else {
        draw_untextured_sprite(canvas, destination, angle_degrees, center, color);
        return;
    };

    let texture_creator = canvas.texture_creator();
    let Ok(mut texture) = texture_creator.create_texture_streaming(
        sdl2::pixels::PixelFormatEnum::RGBA32,
        texture_asset.width.max(1),
        texture_asset.height.max(1),
    ) else {
        draw_untextured_sprite(canvas, destination, angle_degrees, center, color);
        return;
    };
    if texture
        .update(None, &texture_asset.rgba8, texture_asset.width.max(1) as usize * 4)
        .is_err()
    {
        draw_untextured_sprite(canvas, destination, angle_degrees, center, color);
        return;
    }
    texture.set_color_mod(color.r, color.g, color.b);
    texture.set_alpha_mod(color.a);
    texture.set_blend_mode(match render_sprite.canvas.blend_mode {
        crate::components::BlendMode2D::Alpha => sdl2::render::BlendMode::Blend,
        crate::components::BlendMode2D::Additive => sdl2::render::BlendMode::Add,
        crate::components::BlendMode2D::Multiply => sdl2::render::BlendMode::Mod,
    });

    let source = sprite.source_rect_px.map(|source| {
        let min = source.min.max(Vec2::ZERO);
        let max = (source.min + source.size).max(min + Vec2::ONE);
        Rect::new(
            min.x.round() as i32,
            min.y.round() as i32,
            (max.x - min.x).round().max(1.0) as u32,
            (max.y - min.y).round().max(1.0) as u32,
        )
    });
    let _ = canvas.copy_ex(
        &texture,
        source,
        Some(destination),
        angle_degrees,
        Some(center),
        sprite.flip_x,
        sprite.flip_y,
    );
}

fn draw_untextured_sprite(
    canvas: &mut Canvas<Window>,
    destination: Rect,
    angle_degrees: f64,
    center: Point,
    color: Color,
) {
    let angle = (angle_degrees as f32).to_radians();
    let (sin, cos) = angle.sin_cos();
    let origin = Vec2::new(destination.x() as f32, destination.y() as f32);
    let center = Vec2::new(center.x() as f32, center.y() as f32);
    let size = Vec2::new(destination.width() as f32, destination.height() as f32);
    let corners = [
        Vec2::ZERO,
        Vec2::new(size.x, 0.0),
        size,
        Vec2::new(0.0, size.y),
    ];
    let points = corners.map(|corner| {
        let local = corner - center;
        origin + center + Vec2::new(
            cos * local.x - sin * local.y,
            sin * local.x + cos * local.y,
        )
    });
    draw_quad(canvas, &points, color);
}
