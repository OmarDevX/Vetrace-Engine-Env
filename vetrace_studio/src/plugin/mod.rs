use std::any::Any;
use std::error::Error;

use glam::Vec3;
#[cfg(feature = "render_2d")]
use glam::{Vec2, Vec4};
use vetrace_core::{DynamicValue, Engine, FieldPath, InputState, Plugin, Transform};
use vetrace_editor::{
    EditorKeyboardCapture, EditorOnly, EditorPointerCapture, EditorState, EditorViewportBounds,
    UndoHistory,
};
use vetrace_primitives::{spawn_primitive_actor, PrimitiveColliderOptions, PrimitiveSpawnOptions};
#[cfg(feature = "render_2d")]
use vetrace_primitives::{spawn_sprite_2d_actor, Sprite2DSpawnOptions};
use vetrace_project::VetraceProject;
use vetrace_render::{Camera, EguiToolRegistry, RenderSettings};
#[cfg(feature = "render_2d")]
use vetrace_render::{Camera2D, RenderAssets, Sprite2D, TextureAsset};

use crate::assets::StudioAssets;
use crate::builds::StudioBuilds;
use crate::process::{PlayerOutputStream, PlayerProcess, PlayerProcessEvent};
use crate::debugger::StudioDebugger;
use crate::script_assets::{
    create_lua_script, resolve_existing_script, LUA_SCRIPT_COMPONENT_ID, LUA_SCRIPT_FIELD,
};
use crate::script_workspace::StudioScripts;
use crate::project_manager::launch_project_manager;
use crate::scene::{
    active_scene_project_path, capture_authored_scene, create_scene, fill_scene_snapshot, open_scene,
    project_settings, reload_active_scene, restore_authored_scene, save_active_scene,
    save_active_scene_as, save_temporary_play_scene, restore_scene_document,
    AuthoredSceneSnapshot,
};
use crate::protocol::{StudioBridge, StudioCommand, StudioSnapshot};
use crate::recovery::RecoveryManager;
use vetrace_scripting_lua::{LuaDebuggerCommand, LuaDebuggerEvent};
use crate::ui::StudioEguiTool;

mod camera;
mod command_dispatch;
mod command_entity;
mod command_scene;
mod command_debug;
mod command_script;
mod command_asset;
mod command_build;
mod command_project;
mod history;
mod language_context;
mod lifecycle;
mod player_session;
mod scene_actions;
mod script_actions;
mod shortcuts;

use camera::{authored_transform_signature, update_studio_camera};
use language_context::language_context;
use shortcuts::append_keyboard_shortcuts;



#[derive(Clone, Copy, Debug)]
struct StudioCameraState {
    yaw: f32,
    pitch: f32,
    speed: f32,
}

impl Default for StudioCameraState {
    fn default() -> Self { Self { yaw: -2.45, pitch: -0.35, speed: 7.0 } }
}

pub struct StudioPlugin {
    project: VetraceProject,
    bridge: StudioBridge,
    player: PlayerProcess,
    dirty: bool,
    status: String,
    logs: Vec<String>,
    assets: StudioAssets,
    builds: StudioBuilds,
    scripts: StudioScripts,
    spawn_index: u64,
    transform_signature: Option<u64>,
    history: UndoHistory<AuthoredSceneSnapshot>,
    history_ready: bool,
    history_pending: bool,
    history_idle_seconds: f32,
    history_label: String,
    saved_fingerprint: Vec<u8>,
    project_revision: u64,
    recovery: RecoveryManager,
    debugger: StudioDebugger,
}

impl StudioPlugin {
    pub fn new(project: VetraceProject) -> Self {
        let (builds, build_logs) = StudioBuilds::initialize(&project);
        let scripts = StudioScripts::new(&project);
        let recovery = RecoveryManager::new(&project);
        let debugger = StudioDebugger::load(&project);
        Self {
            project,
            bridge: StudioBridge::default(),
            player: PlayerProcess::new(),
            dirty: false,
            status: "Opening project…".to_string(),
            logs: build_logs,
            assets: StudioAssets::default(),
            builds,
            scripts,
            spawn_index: 1,
            transform_signature: None,
            history: UndoHistory::new(128),
            history_ready: false,
            history_pending: false,
            history_idle_seconds: 0.0,
            history_label: String::new(),
            saved_fingerprint: Vec::new(),
            project_revision: 0,
            recovery,
            debugger,
        }
    }

    fn log(&mut self, message: impl Into<String>) {
        let message = message.into();
        println!("vetrace-studio: {message}");
        self.logs.push(message);
        if self.logs.len() > 2_000 {
            let excess = self.logs.len() - 2_000;
            self.logs.drain(..excess);
        }
    }
}
