use super::*;

// Internal bake data types.

#[derive(Clone, Copy, Debug)]
pub(super) struct Tile {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) resolution: u32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct BakeRectAreaLight {
    pub(super) entity: vetrace_core::Entity,
    pub(super) center: Vec3,
    pub(super) axis_u: Vec3,
    pub(super) axis_v: Vec3,
    pub(super) normal: Vec3,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) color: Vec3,
    pub(super) intensity: f32,
    pub(super) samples: u32,
    pub(super) two_sided: bool,
}
