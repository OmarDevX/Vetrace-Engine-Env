use std::path::PathBuf;

use vetrace_asset::AssetId;
use vetrace_build::ExportPreset;
use vetrace_core::{DynamicValue, Entity, FieldPath};
use vetrace_primitives::PrimitiveKind;
use vetrace_project::ProjectManifest;
use vetrace_script_editor::ExternalChangeResolution;
use vetrace_scripting_lua::LuaDebuggerCommand;

#[derive(Clone, Debug)]
pub enum StudioCommand {
    Select(Option<Entity>),
    SetField {
        entity: Entity,
        component: String,
        path: FieldPath,
        value: DynamicValue,
    },
    AddComponent { entity: Entity, component: String },
    RemoveComponent { entity: Entity, component: String },
    Rename { entity: Entity, name: String },
    SpawnEmpty,
    SpawnPrimitive(PrimitiveKind),
    #[cfg(feature = "render_2d")]
    SpawnSprite2D,
    #[cfg(feature = "render_2d")]
    SpawnSprite2DFromAsset { path: PathBuf, screen_position: [f32; 2] },
    #[cfg(feature = "render_2d")]
    SetViewportMode(vetrace_editor::EditorViewportMode),
    DeleteSelected,
    Undo,
    Redo,
    SaveScene,
    NewScene(PathBuf),
    OpenScene(PathBuf),
    SaveSceneAs(PathBuf),
    SetCurrentSceneAsMain,
    SaveProjectSettings(ProjectManifest),
    RecoverSession,
    DiscardRecovery,
    ReloadScene,
    ReloadSceneDiscard,
    SaveAndReload,
    PlayProject,
    DebugProject,
    DebugCommand(LuaDebuggerCommand),
    ToggleBreakpoint { path: PathBuf, line: usize },
    SetDebuggerWatches(Vec<String>),
    SetBreakOnError(bool),
    StopProject,
    OpenScript(PathBuf),
    OpenScriptAt { path: PathBuf, line: usize },
    SaveScript(usize),
    SaveAndCloseScript(usize),
    SaveAllScripts,
    RenameScript { index: usize, project_path: String },
    DeleteScript { index: usize, discard: bool },
    ResolveScriptExternal { path: PathBuf, resolution: ExternalChangeResolution },
    AssignLuaScript { entity: Entity, source: PathBuf },
    CreateLuaScript { entity: Entity, project_path: String },
    RefreshAssets,
    ImportAssetFiles(Vec<PathBuf>),
    ReimportAsset(AssetId),
    ReimportAllAssets,
    ClearAssetCache,
    PruneAssetCache,
    SaveExportPreset(ExportPreset),
    BuildProject { preset: ExportPreset, player_template: PathBuf },
    InstallPlayerTemplate(PathBuf),
    InstallPlayerTemplateArchive(PathBuf),
    DownloadPlayerTemplate(String),
    DownloadCompatiblePlayerTemplate { catalog_url: String, target: vetrace_build::ExportTarget },
    RemovePlayerTemplate(String),
    OpenBuildFolder(PathBuf),
    OpenProjectManager,
    OpenProjectManagerDiscard,
    SaveAndOpenProjectManager,
    Quit,
    QuitDiscard,
    SaveAndQuit,
}
