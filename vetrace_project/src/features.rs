use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FeatureSettings {
    pub rendering: bool,
    pub physics: bool,
    pub audio: bool,
    pub animation: bool,
    pub networking: bool,
    pub ui: bool,
    pub scripting: bool,
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            rendering: true,
            physics: true,
            audio: true,
            animation: true,
            networking: false,
            ui: true,
            scripting: true,
        }
    }
}
