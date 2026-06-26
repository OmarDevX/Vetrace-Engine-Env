//! Fixed timestep tick handling for network simulation.

/// Tick rate in Hertz.
pub const TICK_RATE: f32 = 60.0;

/// Duration of a single tick in seconds.
pub const TICK_DURATION: f32 = 1.0 / TICK_RATE;

/// Input for a specific simulation tick used for client prediction.
#[derive(Debug, Clone)]
pub struct InputFrame {
    /// The tick this input belongs to.
    pub tick: u32,
    /// Raw input data for the tick.
    pub input: crate::net::packets::InputData,
}

/// Simple tick counter used for fixed-step networking.
#[derive(Default)]
pub struct TickManager {
    pub tick: u32,
    accumulator: f32,
}

impl TickManager {
    /// Advance the tick counter by `delta` seconds.
    pub fn advance(&mut self, delta: f32) -> bool {
        self.accumulator += delta;
        if self.accumulator >= TICK_DURATION {
            self.accumulator -= TICK_DURATION;
            self.tick += 1;
            return true;
        }
        false
    }
}
