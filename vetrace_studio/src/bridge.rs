use std::sync::{Arc, Mutex};

use crate::protocol::{StudioCommand, StudioSnapshot, StudioViewportRect};

#[derive(Clone, Default)]
pub struct StudioBridge {
    pub snapshot: Arc<Mutex<StudioSnapshot>>,
    pub commands: Arc<Mutex<Vec<StudioCommand>>>,
    pub pointer_captured: Arc<Mutex<bool>>,
    pub keyboard_captured: Arc<Mutex<bool>>,
    /// Physical-pixel bounds of the unobstructed 3D viewport. The egui pass
    /// updates this after laying out its panels; the next engine frame uses it
    /// before scene picking runs, avoiding one-frame click-through into the
    /// viewport when the user clicks an inspector or hierarchy widget.
    pub viewport_rect: Arc<Mutex<Option<StudioViewportRect>>>,
}

impl StudioBridge {
    pub fn push(&self, command: StudioCommand) {
        if let Ok(mut commands) = self.commands.lock() {
            commands.push(command);
        }
    }

    pub fn drain_commands(&self) -> Vec<StudioCommand> {
        self.commands
            .lock()
            .map(|mut commands| commands.drain(..).collect())
            .unwrap_or_default()
    }
}
