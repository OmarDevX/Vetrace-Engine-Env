use crate::RuntimeMode;

#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub mode: RuntimeMode,
    pub validate_project_files: bool,
    pub load_scene_assets: bool,
    pub start_paused: bool,
    pub stop_on_window_close: bool,
    /// Whether project autoloads and scene-attached Lua scripts are initialized.
    /// Editors disable this so authored scenes can be inspected without running gameplay.
    pub run_project_scripts: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: RuntimeMode::StandaloneGame,
            validate_project_files: true,
            load_scene_assets: true,
            start_paused: false,
            stop_on_window_close: true,
            run_project_scripts: true,
        }
    }
}
