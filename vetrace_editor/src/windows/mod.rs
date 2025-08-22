//! Editor window modules and shared structures.

pub mod main_window;
pub mod sandbox_window;

pub use main_window::MainWindow;
pub use sandbox_window::SandboxWindow;

/// Represents a field for generated components.
#[derive(Clone)]
pub struct NewField {
    pub name: String,
    pub ty_index: usize,
    pub default: String,
}

impl NewField {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            ty_index: 0,
            default: String::new(),
        }
    }
}

impl Default for NewField {
    fn default() -> Self {
        Self::new()
    }
}
