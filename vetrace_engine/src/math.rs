// src/engine/math.rs

pub use glam::{Vec2, Vec3, Vec4, Mat4};

/// Convenience wrapper around [`Vec2::new`].
pub fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Convenience wrapper around [`Vec3::new`].
pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

/// Convenience wrapper around [`Vec4::new`].
pub fn vec4(x: f32, y: f32, z: f32, w: f32) -> Vec4 {
    Vec4::new(x, y, z, w)
}

/// Convert a [`Vec3`] into a `[f32; 3]` array.
pub fn vec3_to_array(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// Convert a `[f32; 3]` array into a [`Vec3`].
pub fn array_to_vec3(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

/// Wrapper around [`Mat4::perspective_rh_gl`].
pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    Mat4::perspective_rh_gl(fov_y, aspect, near, far)
}

/// Create a look-at matrix given eye, center and up vectors.
pub fn look_at(eye: &Vec3, center: &Vec3, up: &Vec3) -> Mat4 {
    Mat4::look_at_rh(*eye, *center, *up)
}

/// Return `m` translated by vector `v`.
pub fn translate(m: &Mat4, v: Vec3) -> Mat4 {
    *m * Mat4::from_translation(v)
}

/// Return `m` scaled by vector `v`.
pub fn scale(m: &Mat4, v: Vec3) -> Mat4 {
    *m * Mat4::from_scale(v)
}

/// Return `m` rotated around `axis` by `angle_rad` radians.
pub fn rotate(m: &Mat4, angle_rad: f32, axis: Vec3) -> Mat4 {
    *m * Mat4::from_axis_angle(axis.normalize_or_zero(), angle_rad)
}
