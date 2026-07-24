use std::collections::HashSet;

/// Runtime-neutral input state stored as an optional core resource.
///
/// Window/platform crates translate their real input events into these string
/// identifiers. This keeps `vetrace_core` free of SDL/winit dependencies while
/// still giving scripting/gameplay plugins a stable input surface.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    keys_down: HashSet<String>,
    keys_pressed: HashSet<String>,
    keys_released: HashSet<String>,
    mouse_buttons_down: HashSet<String>,
    mouse_buttons_pressed: HashSet<String>,
    mouse_buttons_released: HashSet<String>,
    mouse_position: (f32, f32),
    mouse_delta: (f32, f32),
    mouse_wheel_delta: (f32, f32),
    text_input: String,
    quit_requested: bool,
}

impl InputState {
    pub fn new() -> Self { Self::default() }

    /// Clears frame-local transitions while preserving held keys/buttons.
    ///
    /// Platform/render crates should call this once before they pump their event
    /// queue for a new frame. Gameplay then reads the resulting state on the
    /// following update, keeping the core loop dependency-free.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_buttons_pressed.clear();
        self.mouse_buttons_released.clear();
        self.mouse_delta = (0.0, 0.0);
        self.mouse_wheel_delta = (0.0, 0.0);
        self.text_input.clear();
        self.quit_requested = false;
    }

    pub fn set_key_down(&mut self, key: impl Into<String>, down: bool) {
        let key = key.into();
        if down {
            if self.keys_down.insert(key.clone()) {
                self.keys_pressed.insert(key);
            }
        } else if self.keys_down.remove(&key) {
            self.keys_released.insert(key);
        }
    }

    pub fn set_mouse_button_down(&mut self, button: impl Into<String>, down: bool) {
        let button = button.into();
        if down {
            if self.mouse_buttons_down.insert(button.clone()) {
                self.mouse_buttons_pressed.insert(button);
            }
        } else if self.mouse_buttons_down.remove(&button) {
            self.mouse_buttons_released.insert(button);
        }
    }

    pub fn set_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_position = (x, y);
    }

    pub fn add_mouse_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_delta.0 += dx;
        self.mouse_delta.1 += dy;
    }

    pub fn add_mouse_wheel_delta(&mut self, dx: f32, dy: f32) {
        self.mouse_wheel_delta.0 += dx;
        self.mouse_wheel_delta.1 += dy;
    }

    pub fn push_text_input(&mut self, text: &str) {
        self.text_input.extend(text.chars().filter(|ch| !ch.is_control()));
    }

    pub fn request_quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn is_key_down(&self, key: &str) -> bool { self.keys_down.contains(key) }
    pub fn was_key_pressed(&self, key: &str) -> bool { self.keys_pressed.contains(key) }
    pub fn was_key_released(&self, key: &str) -> bool { self.keys_released.contains(key) }

    pub fn is_mouse_button_down(&self, button: &str) -> bool { self.mouse_buttons_down.contains(button) }
    pub fn was_mouse_button_pressed(&self, button: &str) -> bool { self.mouse_buttons_pressed.contains(button) }
    pub fn was_mouse_button_released(&self, button: &str) -> bool { self.mouse_buttons_released.contains(button) }

    pub fn mouse_position(&self) -> (f32, f32) { self.mouse_position }
    pub fn mouse_delta(&self) -> (f32, f32) { self.mouse_delta }
    pub fn mouse_wheel_delta(&self) -> (f32, f32) { self.mouse_wheel_delta }
    pub fn text_input(&self) -> &str { &self.text_input }
    pub fn quit_requested(&self) -> bool { self.quit_requested }
}
