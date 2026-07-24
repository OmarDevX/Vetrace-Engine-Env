use super::*;

// Deterministic sampling and attenuation helpers.

pub(super) fn radical_inverse_vdc(mut bits: u32) -> f32 {
    bits = (bits << 16) | (bits >> 16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xaaaa_aaaa) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xcccc_cccc) >> 2);
    bits = ((bits & 0x0f0f_0f0f) << 4) | ((bits & 0xf0f0_f0f0) >> 4);
    bits = ((bits & 0x00ff_00ff) << 8) | ((bits & 0xff00_ff00) >> 8);
    bits as f32 * 2.328_306_4e-10
}

pub(super) fn fibonacci_direction(index: u32, count: u32) -> Vec3 {
    let golden_angle = PI * (3.0 - 5.0_f32.sqrt());
    let y = 1.0 - 2.0 * (index as f32 + 0.5) / count as f32;
    let radius = (1.0 - y * y).max(0.0).sqrt();
    let theta = golden_angle * index as f32;
    Vec3::new(theta.cos() * radius, y, theta.sin() * radius)
}

pub(super) fn range_attenuation(distance: f32, range: f32) -> f32 {
    if range <= 0.0 { return 1.0; }
    let x = (1.0 - distance / range.max(0.0001)).clamp(0.0, 1.0);
    x * x
}

pub(super) fn grid_fraction(index: u32, count: u32) -> f32 {
    if count <= 1 { 0.5 } else { index as f32 / (count - 1) as f32 }
}
