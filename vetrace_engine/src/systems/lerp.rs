use crate::components::components::{Lerp, LerpData, LoopMode, LerpState, Interpolate};
use crate::engine::engine::Engine;
use crate::Behaviour;

pub struct LerpSystem;

impl Default for LerpSystem {
    fn default() -> Self { Self }
}

fn update_lerp<T: Interpolate + Default>(lerp: &mut LerpData<T>, dt: f32) {
    match lerp.state {
        LerpState::PlayingForward => {
            lerp.progress += lerp.speed * dt;
            if lerp.progress >= 1.0 {
                match lerp.loop_mode {
                    LoopMode::Loop => lerp.progress = 0.0,
                    LoopMode::PingPong => {
                        lerp.progress = 1.0;
                        lerp.state = LerpState::PlayingBackward;
                    }
                    LoopMode::None => {
                        lerp.progress = 1.0;
                        lerp.state = LerpState::Stopped;
                    }
                }
            }
        }
        LerpState::PlayingBackward => {
            lerp.progress -= lerp.speed * dt;
            if lerp.progress <= 0.0 {
                match lerp.loop_mode {
                    LoopMode::Loop => lerp.progress = 1.0,
                    LoopMode::PingPong => {
                        lerp.progress = 0.0;
                        lerp.state = LerpState::PlayingForward;
                    }
                    LoopMode::None => {
                        lerp.progress = 0.0;
                        lerp.state = LerpState::Stopped;
                    }
                }
            }
        }
        LerpState::Paused | LerpState::Stopped => {}
    }
}

impl Behaviour for LerpSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut q = engine.world.query_mut::<Lerp>();
        for (_, lerp) in q.iter_mut() {
            match lerp {
                Lerp::F32(l) => update_lerp(l, delta),
                Lerp::Vec3(l) => update_lerp(l, delta),
            }
        }
    }
}
