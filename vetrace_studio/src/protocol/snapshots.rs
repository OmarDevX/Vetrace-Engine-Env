use std::path::PathBuf;

use vetrace_asset::{AssetDiagnosticSeverity, AssetId, AssetKind, AssetStatus};
use vetrace_build::ExportPreset;
use vetrace_core::{ComponentSchema, DynamicValue, Entity, FieldPath, FieldSchema};
use vetrace_project::ProjectManifest;
use vetrace_script_editor::LanguageContext;

use crate::debugger::DebuggerSnapshot;

#[derive(Clone, Debug, Default)]
pub struct EntityRow {
    pub entity: Entity,
    pub name: String,
    pub depth: usize,
}

#[derive(Clone, Debug)]
pub struct ReflectedFieldSnapshot {
    pub schema: FieldSchema,
    pub path: FieldPath,
    pub value: DynamicValue,
}

#[derive(Clone, Debug)]
pub struct ReflectedComponentSnapshot {
    pub schema: ComponentSchema,
    pub fields: Vec<ReflectedFieldSnapshot>,
}



#[derive(Clone, Debug)]
pub struct DraggedAsset {
    pub path: PathBuf,
    pub file_name: String,
    pub kind: AssetKind,
}

#[derive(Clone, Debug)]
pub struct AssetRow {
    pub id: AssetId,
    pub path: String,
    pub file_name: String,
    pub kind: AssetKind,
    pub status: AssetStatus,
    pub size: u64,
    pub hash: String,
    pub importer: Option<String>,
    pub outputs: Vec<String>,
    pub dependencies: Vec<(String, bool)>,
    pub error: Option<String>,
    pub thumbnail: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct AssetDiagnosticRow {
    pub severity: AssetDiagnosticSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct AssetCacheSnapshot {
    pub imported_assets: usize,
    pub imported_files: usize,
    pub bytes: u64,
    pub orphan_directories: usize,
}


#[derive(Clone, Debug, Default)]
pub struct BuildReportSnapshot {
    pub output_directory: PathBuf,
    pub executable: PathBuf,
    pub package: PathBuf,
    pub package_entries: usize,
    pub package_bytes: u64,
    pub package_blake3: String,
    pub warnings: Vec<String>,
}


#[derive(Clone, Debug, Default)]
pub struct PlayerTemplateSnapshot {
    pub id: String,
    pub engine_version: String,
    pub target: String,
    pub binary: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct BuildSnapshot {
    pub presets: Vec<ExportPreset>,
    pub default_preset: String,
    pub detected_player_template: Option<PathBuf>,
    pub installed_templates: Vec<PlayerTemplateSnapshot>,
    pub running: bool,
    pub status: String,
    pub last_report: Option<BuildReportSnapshot>,
}

#[derive(Clone, Debug, Default)]
pub struct StudioSnapshot {
    pub project_name: String,
    pub project_root: PathBuf,
    pub scene_path: String,
    pub dirty: bool,
    pub status: String,
    pub selected: Option<Entity>,
    pub selected_name: String,
    pub entities: Vec<EntityRow>,
    pub components: Vec<ReflectedComponentSnapshot>,
    pub addable_components: Vec<ComponentSchema>,
    pub assets: Vec<AssetRow>,
    pub asset_diagnostics: Vec<AssetDiagnosticRow>,
    pub asset_cache: AssetCacheSnapshot,
    pub builds: BuildSnapshot,
    pub logs: Vec<String>,
    pub scripts_dirty: bool,
    pub language_context: LanguageContext,
    pub player_running: bool,
    pub debugger: DebuggerSnapshot,
    pub can_undo: bool,
    pub can_redo: bool,
    pub project_settings: Vec<(String, String)>,
    pub project_manifest: ProjectManifest,
    pub project_revision: u64,
    pub recovery_available: bool,
    #[cfg(feature = "render_2d")]
    pub viewport_mode: vetrace_editor::EditorViewportMode,
}

pub use vetrace_editor::EditorViewportRect as StudioViewportRect;
