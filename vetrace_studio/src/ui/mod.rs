use vetrace_core::{DynamicValue, FieldKind, FieldPath, FieldSchema};
use vetrace_render::{egui, EguiTool, EguiToolContext};
use vetrace_scripting_lua::LuaDebuggerCommand;
use vetrace_project::{
    AxisDirection, GiMode, InputAction, ProjectManifest, ProjectPath, RenderingBackend,
    ScriptLanguage, ShadowQuality,
};

use crate::asset_browser::AssetBrowserState;
use crate::build_panel::BuildPanelState;
use crate::script_asset_ui::ScriptAssetUi;
use crate::script_assets::{is_lua_script_field, LUA_SCRIPT_COMPONENT_ID};
use crate::script_panel::ScriptEditorPanel;
use crate::script_workspace::{parse_console_script_location, StudioScripts};
use crate::protocol::{
    ReflectedComponentSnapshot, ReflectedFieldSnapshot, StudioBridge, StudioCommand,
    StudioSnapshot, StudioViewportRect,
};

mod bottom_panel;
mod confirmation;
mod console;
mod dynamic_value;
mod inspector;
mod inspector_panel;
mod project_settings;
mod scene_tree;
mod settings_helpers;
mod shell;
mod toolbar;
mod viewport_state;

use dynamic_value::draw_dynamic_value;
use settings_helpers::{binding_list, empty_project_path, enum_combo, settings_text};


pub struct StudioEguiTool {
    bridge: StudioBridge,
    selected_add_component: String,
    bottom_tab: BottomTab,
    asset_browser: AssetBrowserState,
    build_panel: BuildPanelState,
    scripts: StudioScripts,
    script_panel: ScriptEditorPanel,
    confirmation: Option<Confirmation>,
    script_assets: ScriptAssetUi,
    project_draft: Option<ProjectManifest>,
    project_revision: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BottomTab {
    #[default]
    Assets,
    Console,
    Scripts,
    Project,
    Build,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Confirmation {
    Reload,
    ProjectManager,
    Quit,
}

impl StudioEguiTool {
    pub fn new(bridge: StudioBridge, scripts: StudioScripts) -> Self {
        Self {
            bridge,
            selected_add_component: String::new(),
            bottom_tab: BottomTab::Assets,
            asset_browser: AssetBrowserState::default(),
            build_panel: BuildPanelState::default(),
            scripts,
            script_panel: ScriptEditorPanel::default(),
            confirmation: None,
            script_assets: ScriptAssetUi::default(),
            project_draft: None,
            project_revision: u64::MAX,
        }
    }

    fn command(&self, command: StudioCommand) { self.bridge.push(command); }
}
