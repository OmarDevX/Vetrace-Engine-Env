use vetrace_scripting_lua::LuaModManager;
use std::collections::BTreeMap;

pub struct ShooterModRuntime {
    pub manager: LuaModManager,
    pub status: String,
    pub watch_elapsed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShooterModEffects {
    pub movement_multiplier: f32,
    pub jump_multiplier: f32,
    pub gravity_scale: f32,
    pub vignette_strength: Option<f32>,
}

impl Default for ShooterModEffects {
    fn default() -> Self {
        Self {
            movement_multiplier: 1.0,
            jump_multiplier: 1.0,
            gravity_scale: 1.0,
            vignette_strength: None,
        }
    }
}

#[derive(Default)]
pub struct ShooterModContributions {
    pub by_mod: BTreeMap<String, ShooterModEffects>,
}
