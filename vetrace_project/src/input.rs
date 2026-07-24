use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputMap {
    pub actions: BTreeMap<String, InputAction>,
}

impl InputMap {
    pub fn action(&self, name: &str) -> Option<&InputAction> {
        self.actions.get(name)
    }

    pub fn action_mut(&mut self, name: &str) -> Option<&mut InputAction> {
        self.actions.get_mut(name)
    }

    pub fn insert(&mut self, name: impl Into<String>, action: InputAction) -> Option<InputAction> {
        self.actions.insert(name.into(), action)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct InputAction {
    /// Physical key names, matching Vetrace's canonical names such as `W`,
    /// `ArrowUp`, `Space`, `Shift`, and `F10`.
    pub keys: Vec<String>,
    /// Mouse button names such as `Left`, `Right`, or `Middle`.
    pub mouse_buttons: Vec<String>,
    /// Platform-independent gamepad button names.
    pub gamepad_buttons: Vec<String>,
    /// Analog axis bindings.
    pub axes: Vec<InputAxisBinding>,
    /// Analog values whose magnitude is below this value are ignored.
    pub dead_zone: f32,
}

impl Default for InputAction {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            mouse_buttons: Vec::new(),
            gamepad_buttons: Vec::new(),
            axes: Vec::new(),
            dead_zone: 0.15,
        }
    }
}

impl InputAction {
    pub fn is_unbound(&self) -> bool {
        self.keys.is_empty()
            && self.mouse_buttons.is_empty()
            && self.gamepad_buttons.is_empty()
            && self.axes.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputAxisBinding {
    pub axis: String,
    #[serde(default)]
    pub direction: AxisDirection,
    #[serde(default = "default_axis_scale")]
    pub scale: f32,
    #[serde(default)]
    pub gamepad: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisDirection {
    Negative,
    #[default]
    Positive,
}

fn default_axis_scale() -> f32 {
    1.0
}
