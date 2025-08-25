use crate::input::{window::WindowManager, Input};
use crate::systems::free_flight::FreeFlightState;

/// Manages input handling and window management
pub struct InputManager {
    pub input: Input,
    pub window: WindowManager,
    pub sdl_context: sdl2::Sdl,
    pub free_flight: FreeFlightState,
}

impl InputManager {
    pub fn new(
        input: Input,
        window: WindowManager,
        sdl_context: sdl2::Sdl,
        free_flight: FreeFlightState,
    ) -> Self {
        Self {
            input,
            window,
            sdl_context,
            free_flight,
        }
    }

    /// Get input reference
    pub fn input(&self) -> &Input {
        &self.input
    }

    /// Get mutable input reference
    pub fn input_mut(&mut self) -> &mut Input {
        &mut self.input
    }

    /// Get window manager reference
    pub fn window(&self) -> &WindowManager {
        &self.window
    }

    /// Get mutable window manager reference
    pub fn window_mut(&mut self) -> &mut WindowManager {
        &mut self.window
    }

    /// Get SDL context reference
    pub fn sdl_context(&self) -> &sdl2::Sdl {
        &self.sdl_context
    }

    /// Get free flight state reference
    pub fn free_flight(&self) -> &FreeFlightState {
        &self.free_flight
    }

    /// Get mutable free flight state reference
    pub fn free_flight_mut(&mut self) -> &mut FreeFlightState {
        &mut self.free_flight
    }
}
