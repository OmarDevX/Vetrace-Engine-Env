pub mod window;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use crate::events::Event;
use std::collections::HashSet;

/// Stores keyboard and mouse state for the current frame.
///
/// The struct keeps track of which keys and mouse buttons are
/// currently held down as well as which were pressed or released
/// during the last frame. `mouse_x` and `mouse_y` contain the latest
/// cursor position and `mouse_wheel` stores the scroll wheel delta for
/// the frame.
#[derive(Default)]
pub struct Input {
    /// Keys that are currently held down.
    pub keys_down: HashSet<Keycode>,
    /// Keys that were pressed during the current frame.
    pub keys_pressed: HashSet<Keycode>,
    /// Keys that were released during the current frame.
    pub keys_released: HashSet<Keycode>,
    /// Mouse buttons that are currently held down.
    pub mouse_buttons_down: HashSet<MouseButton>,
    /// Current mouse x position in window coordinates.
    pub mouse_x: i32,
    /// Current mouse y position in window coordinates.
    pub mouse_y: i32,
    /// Mouse movement delta on the x axis accumulated this frame.
    pub mouse_delta_x: i32,
    /// Mouse movement delta on the y axis accumulated this frame.
    pub mouse_delta_y: i32,
    /// Whether the mouse is currently captured (relative mode).
    pub mouse_captured: bool,
    /// Scroll wheel delta accumulated this frame.
    pub mouse_wheel: i32,
    /// Event triggered when a key is pressed.
    #[allow(missing_docs)]
    pub on_key_down: Event<Keycode>,
    /// Event triggered when a key is released.
    #[allow(missing_docs)]
    pub on_key_up: Event<Keycode>,
    /// Event triggered when a mouse button is pressed.
    #[allow(missing_docs)]
    pub on_mouse_down: Event<MouseButton>,
    /// Event triggered when a mouse button is released.
    #[allow(missing_docs)]
    pub on_mouse_up: Event<MouseButton>,
}

impl Input {
    /// Create a new [`Input`] instance with all state cleared.
    pub fn new() -> Self {
        Input {
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            keys_released: HashSet::new(),
            mouse_buttons_down: HashSet::new(),
            mouse_x: 0,
            mouse_y: 0,
            mouse_delta_x: 0,
            mouse_delta_y: 0,
            mouse_captured: false,
            mouse_wheel: 0,
            on_key_down: Event::new(),
            on_key_up: Event::new(),
            on_mouse_down: Event::new(),
            on_mouse_up: Event::new(),
        }
    }

    /// Reset per-frame values such as `keys_pressed`, `keys_released` and
    /// `mouse_wheel`.
    ///
    /// This should be called once at the start of every frame before any
    /// events are processed.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.mouse_wheel = 0;
        self.mouse_delta_x = 0;
        self.mouse_delta_y = 0;
    }

    /// Check if a key is currently pressed (by string name)
    pub fn is_key_pressed(&self, key_name: &str) -> bool {
        match key_name {
            "Escape" => self.keys_down.contains(&Keycode::Escape),
            "Space" => self.keys_down.contains(&Keycode::Space),
            "W" | "w" => self.keys_down.contains(&Keycode::W),
            "A" | "a" => self.keys_down.contains(&Keycode::A),
            "S" | "s" => self.keys_down.contains(&Keycode::S),
            "D" | "d" => self.keys_down.contains(&Keycode::D),
            _ => false,
        }
    }

    /// Check if a key is currently pressed (by keycode)
    pub fn is_key_down(&self, keycode: Keycode) -> bool {
        self.keys_down.contains(&keycode)
    }

    /// Update the input state based on a single SDL2 event.
    ///
    /// This function should be invoked for every event polled from SDL2 in
    /// order to keep the input state in sync with the window system.
    /// Keyboard, mouse button, motion and wheel events are handled.
    pub fn update(&mut self, event: &sdl2::event::Event) {
        match event {
            sdl2::event::Event::KeyDown { keycode: Some(k), repeat, .. } => {
                if !*repeat {
                    if !self.keys_down.contains(k) {
                        self.keys_pressed.insert(*k);
                        self.on_key_down.emit(*k);
                    }
                    self.keys_down.insert(*k);
                }
            }
            sdl2::event::Event::KeyUp { keycode: Some(k), .. } => {
                self.keys_down.remove(k);
                self.keys_released.insert(*k);
                self.on_key_up.emit(*k);
            }
            sdl2::event::Event::MouseButtonDown { mouse_btn, .. } => {
                self.mouse_buttons_down.insert(*mouse_btn);
                self.on_mouse_down.emit(*mouse_btn);
            }
            sdl2::event::Event::MouseButtonUp { mouse_btn, .. } => {
                self.mouse_buttons_down.remove(mouse_btn);
                self.on_mouse_up.emit(*mouse_btn);
            }
             sdl2::event::Event::MouseMotion { x, y, xrel, yrel, .. } => {
                self.mouse_x = *x;
                self.mouse_y = *y;
                self.mouse_delta_x += *xrel;
                self.mouse_delta_y += *yrel;
            }
            sdl2::event::Event::MouseWheel { y, .. } => {
                self.mouse_wheel = *y;
            }
            _ => {}
        }
    }



    /// Returns `true` if the key was pressed during this frame.
    pub fn was_key_pressed(&self, key: Keycode) -> bool {
        self.keys_pressed.contains(&key)
    }

    /// Returns `true` if the key was released during this frame.
    pub fn was_key_released(&self, key: Keycode) -> bool {
        self.keys_released.contains(&key)
    }

    /// Returns `true` if the specified mouse button is currently held down.
    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Current mouse position in window coordinates.
    pub fn mouse_position(&self) -> (i32, i32) {
        (self.mouse_x, self.mouse_y)
    }

    /// Scroll wheel delta accumulated during this frame.
    pub fn get_mouse_wheel(&self) -> i32 {
        self.mouse_wheel
    }
    /// Alias for [`is_key_down`].
    pub fn is_key_held(&self, key: Keycode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Mouse movement delta accumulated during this frame.
    pub fn mouse_delta(&self) -> (i32, i32) {
        (self.mouse_delta_x, self.mouse_delta_y)
    }
}
