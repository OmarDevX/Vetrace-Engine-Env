use crate::engine::engine::Engine;
pub trait Behaviour {
    fn start(&mut self, engine: &mut Engine) {}
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {}
}
