use super::*;

pub(crate) fn draw_background(canvas: &mut Canvas<Window>, frame: &RenderFrame, width: u32, height: u32) {
    let clear = frame.settings.clear_color;
    let top = frame
        .atmosphere
        .as_ref()
        .map(|a| a.sky_tint * a.intensity.max(0.0))
        .unwrap_or(Vec3::new(clear[0], clear[1], clear[2]));
    let bottom = frame
        .atmosphere
        .as_ref()
        .map(|a| a.ground_tint * a.intensity.max(0.0))
        .unwrap_or(Vec3::new(clear[0] * 0.7, clear[1] * 0.7, clear[2] * 0.7));

    for y in 0..height.max(1) {
        let t = y as f32 / height.max(1) as f32;
        let color = top.lerp(bottom, t).clamp(Vec3::ZERO, Vec3::ONE);
        canvas.set_draw_color(Color::RGB(
            (color.x * 255.0) as u8,
            (color.y * 255.0) as u8,
            (color.z * 255.0) as u8,
        ));
        let _ = canvas.draw_line(Point::new(0, y as i32), Point::new(width as i32, y as i32));
    }
}
