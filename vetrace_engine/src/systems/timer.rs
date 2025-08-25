use crate::{components::components::Timer, engine::engine::Engine, Behaviour};
use mlua::Value as LuaValue;

pub struct TimerSystem;

impl Default for TimerSystem {
    fn default() -> Self {
        Self
    }
}

impl Behaviour for TimerSystem {
    fn update(&mut self, engine: &mut Engine, delta: f32) {
        let mut timeouts = Vec::new();
        {
            let mut query = engine.world.query_mut::<Timer>();
            for (entity, timer) in query.iter_mut() {
                if timer.autostart && timer.is_stopped() {
                    timer.start();
                }
                if timer.is_stopped() || timer.paused {
                    continue;
                }
                timer.time_left -= delta;
                if timer.time_left <= 0.0 {
                    timeouts.push(*entity);
                    if timer.one_shot {
                        timer.stop();
                    } else {
                        timer.time_left += timer.wait_time;
                    }
                }
            }
        }
        for e in timeouts {
            engine.emit_signal(e, "timeout", LuaValue::Nil);
        }
    }
}
