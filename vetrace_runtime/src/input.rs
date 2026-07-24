use vetrace_core::InputState;
use vetrace_project::{InputAction, InputMap};

#[derive(Clone, Debug, Default)]
pub struct RuntimeInputMap {
    map: InputMap,
}

impl RuntimeInputMap {
    pub fn new(map: InputMap) -> Self { Self { map } }
    pub fn map(&self) -> &InputMap { &self.map }
    pub fn action(&self, name: &str) -> Option<&InputAction> { self.map.action(name) }

    pub fn is_down(&self, input: &InputState, name: &str) -> bool {
        self.action(name).is_some_and(|action| {
            action.keys.iter().any(|key| input.is_key_down(key))
                || action.mouse_buttons.iter().any(|button| input.is_mouse_button_down(button))
        })
    }

    pub fn was_pressed(&self, input: &InputState, name: &str) -> bool {
        self.action(name).is_some_and(|action| {
            action.keys.iter().any(|key| input.was_key_pressed(key))
                || action.mouse_buttons.iter().any(|button| input.was_mouse_button_pressed(button))
        })
    }

    pub fn was_released(&self, input: &InputState, name: &str) -> bool {
        self.action(name).is_some_and(|action| {
            action.keys.iter().any(|key| input.was_key_released(key))
                || action.mouse_buttons.iter().any(|button| input.was_mouse_button_released(button))
        })
    }

    /// Digital contribution of key/mouse bindings. Gamepad axes remain part of
    /// the project format and will be evaluated when the platform input layer
    /// exposes analog values in `InputState`.
    pub fn digital_value(&self, input: &InputState, name: &str) -> f32 {
        if self.is_down(input, name) { 1.0 } else { 0.0 }
    }
}
