use std::any::Any;
use std::error::Error;

use crate::app::Plugin;
use crate::components::builtins::Timer;
use crate::engine::Engine;
use crate::Stage;

/// Advances all core `Timer` components.
pub fn tick_timers(engine: &mut Engine, dt: f32) {
    for (_entity, timer) in engine.raw_world_mut().query_mut::<Timer>() {
        timer.elapsed += dt.max(0.0);
        if timer.duration > 0.0 && timer.elapsed >= timer.duration {
            if timer.repeating {
                timer.elapsed %= timer.duration;
            } else {
                timer.elapsed = timer.duration;
            }
        }
    }
}

/// Legacy compatibility plugin. `Engine::new` installs timer ticking by default.
pub struct TimerPlugin;

impl TimerPlugin {
    pub fn new() -> Self { Self }
}

impl Default for TimerPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for TimerPlugin {
    fn name(&self) -> &'static str { "core_timer" }
    fn update_stage(&self) -> Stage { Stage::FixedUpdate }

    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn Error>> { Ok(()) }

    fn update(&mut self, _engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        // Engine::new owns the scheduled timer system; avoid double ticking.
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
