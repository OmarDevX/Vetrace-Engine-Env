use super::*;

pub(crate) fn opaque(color: Color) -> Color {
    Color::RGBA(color.r, color.g, color.b, 255)
}

pub(crate) fn darken(color: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    Color::RGBA(
        (color.r as f32 * factor) as u8,
        (color.g as f32 * factor) as u8,
        (color.b as f32 * factor) as u8,
        color.a,
    )
}

pub(crate) fn draw_quad(canvas: &mut Canvas<Window>, points: &[Vec2; 4], color: Color) {
    fill_triangle(canvas, points[0], points[1], points[2], color);
    fill_triangle(canvas, points[0], points[2], points[3], color);
}

fn fill_triangle(canvas: &mut Canvas<Window>, a: Vec2, b: Vec2, c: Vec2, color: Color) {
    let min_y = a.y.min(b.y).min(c.y).floor().max(-4096.0) as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min(4096.0) as i32;
    canvas.set_draw_color(color);

    for y in min_y..=max_y {
        let yf = y as f32 + 0.5;
        let mut xs = [0.0_f32; 3];
        let mut count = 0usize;
        add_edge_intersection(a, b, yf, &mut xs, &mut count);
        add_edge_intersection(b, c, yf, &mut xs, &mut count);
        add_edge_intersection(c, a, yf, &mut xs, &mut count);
        if count >= 2 {
            xs[..count].sort_by(|x0, x1| x0.total_cmp(x1));
            let x0 = xs[0].floor() as i32;
            let x1 = xs[count - 1].ceil() as i32;
            let _ = canvas.draw_line(Point::new(x0, y), Point::new(x1, y));
        }
    }
}

fn add_edge_intersection(a: Vec2, b: Vec2, y: f32, xs: &mut [f32; 3], count: &mut usize) {
    let crosses = (a.y <= y && b.y > y) || (b.y <= y && a.y > y);
    if !crosses || (a.y - b.y).abs() <= f32::EPSILON || *count >= xs.len() {
        return;
    }
    let t = (y - a.y) / (b.y - a.y);
    xs[*count] = a.x + t * (b.x - a.x);
    *count += 1;
}

pub(crate) fn draw_polyline(canvas: &mut Canvas<Window>, points: &[Vec2; 4], color: Color, closed: bool) {
    canvas.set_draw_color(color);
    for window in points.windows(2) {
        let _ = canvas.draw_line(to_point(window[0]), to_point(window[1]));
    }
    if closed {
        let _ = canvas.draw_line(to_point(points[points.len() - 1]), to_point(points[0]));
    }
}

pub(crate) fn draw_wire_cube(canvas: &mut Canvas<Window>, points: &[Vec2; 8], color: Color) {
    canvas.set_draw_color(color);
    for (a, b) in cube_edges() {
        let _ = canvas.draw_line(to_point(points[a]), to_point(points[b]));
    }
}

fn cube_edges() -> [(usize, usize); 12] {
    [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
    ]
}

fn to_point(point: Vec2) -> Point {
    Point::new(point.x.round() as i32, point.y.round() as i32)
}
