use glam::{Quat, Vec3};

use crate::{
    components::components::{Transform, Rotate},
    engine::engine::Engine,
    Behaviour,
};

pub struct Rotator;

impl Rotator {
    pub fn new() -> Self {
        Rotator
    }
}

impl Behaviour for Rotator {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        // Rotate all entities that have Transform and Rotate components
        for (_entity, transform, rotate) in engine.world.query2_mut::<Transform, Rotate>() {
            let q = Quat::from_xyzw(
                transform.orientation[0],
                transform.orientation[1],
                transform.orientation[2],
                transform.orientation[3],
            );
            let rot_y = Quat::from_axis_angle(Vec3::Y, rotate.speed * delta);
            let new_q = rot_y * q;
            transform.orientation = [new_q.x, new_q.y, new_q.z, new_q.w];
        }
    }
}