use std::collections::BTreeMap;

use vetrace_core::InputState;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LuaInputAction {
    pub keys: Vec<String>,
    pub mouse_buttons: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LuaInputMap {
    actions: BTreeMap<String, LuaInputAction>,
}

impl LuaInputMap {
    pub fn new(actions: BTreeMap<String, LuaInputAction>) -> Self { Self { actions } }

    pub fn insert(&mut self, name: impl Into<String>, action: LuaInputAction) -> Option<LuaInputAction> {
        self.actions.insert(name.into(), action)
    }

    pub fn action(&self, name: &str) -> Option<&LuaInputAction> { self.actions.get(name) }

    pub fn action_down(&self, input: &InputState, name: &str) -> bool {
        self.action(name).is_some_and(|action| {
            action.keys.iter().any(|key| input.is_key_down(key))
                || action.mouse_buttons.iter().any(|button| input.is_mouse_button_down(button))
        })
    }

    pub fn action_pressed(&self, input: &InputState, name: &str) -> bool {
        self.action(name).is_some_and(|action| {
            action.keys.iter().any(|key| input.was_key_pressed(key))
                || action.mouse_buttons.iter().any(|button| input.was_mouse_button_pressed(button))
        })
    }

    pub fn action_released(&self, input: &InputState, name: &str) -> bool {
        self.action(name).is_some_and(|action| {
            action.keys.iter().any(|key| input.was_key_released(key))
                || action.mouse_buttons.iter().any(|button| input.was_mouse_button_released(button))
        })
    }
}
