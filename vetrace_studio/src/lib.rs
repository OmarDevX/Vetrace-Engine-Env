mod args;
mod asset_browser;
mod assets;
mod bridge;
mod build_panel;
mod builds;
mod debugger;
mod plugin;
mod process;
mod project_manager;
mod recovery;
mod scene;
mod script_asset_ui;
mod script_assets;
mod script_panel;
mod script_workspace;
mod protocol;
mod ui;

use std::error::Error;
use std::path::Path;

use args::StudioArgs;
use plugin::StudioPlugin;
use vetrace_editor::{EditorConfig, EditorPlugin};
use vetrace_project::{migrate_project, VetraceProject};
use vetrace_runtime::{RuntimeMode, VetraceRuntime};

pub fn run_from_env() -> Result<(), Box<dyn Error>> {
    let args = match StudioArgs::parse() {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            if message.starts_with("Usage:") { return Ok(()); }
            return Err(message.into());
        }
    };

    if args.project_manager || args.project.is_none() {
        return project_manager::run_project_manager(args.max_frames);
    }

    run_editor(args.project.as_deref().expect("project path checked"), args.max_frames)
}

pub fn run_editor(project_path: &Path, max_frames: Option<usize>) -> Result<(), Box<dyn Error>> {
    let discovered = vetrace_project::find_project_root(project_path).unwrap_or_else(|_| project_path.to_path_buf());
    match migrate_project(&discovered) {
        Ok(report) if report.changed() => {
            eprintln!("vetrace-studio: migrated project format {} -> {}", report.from_version, report.to_version);
            for change in report.changes { eprintln!("vetrace-studio: migration: {change}"); }
        }
        Ok(_) => {}
        Err(error) => eprintln!("vetrace-studio: project migration skipped: {error}"),
    }
    let project = VetraceProject::discover(project_path)
        .or_else(|_| VetraceProject::load(project_path))?;
    if let Err(error) = project_manager::record_recent_project(&project) {
        eprintln!("vetrace-studio: failed to update recent projects: {error}");
    }

    // Studio installs the complete built-in editing surface even when a game
    // disables a subsystem for its exported runtime. The original manifest is
    // kept by Studio and is never overwritten by these editor-only overrides.
    let mut editor_project = project.clone();
    let editor_width = editor_project.manifest().application.width.max(1280);
    let editor_height = editor_project.manifest().application.height.max(720);
    editor_project.manifest_mut().application.title =
        format!("Vetrace Studio — {}", project.manifest().project.name);
    editor_project.manifest_mut().application.width = editor_width;
    editor_project.manifest_mut().application.height = editor_height;
    editor_project.manifest_mut().application.cursor_grab = false;
    editor_project.manifest_mut().application.cursor_visible = true;
    editor_project.manifest_mut().features.rendering = true;
    editor_project.manifest_mut().features.ui = true;
    editor_project.manifest_mut().features.physics = true;
    editor_project.manifest_mut().features.audio = true;
    editor_project.manifest_mut().features.animation = true;
    editor_project.manifest_mut().features.scripting = true;

    let studio_plugin = StudioPlugin::new(project.clone());
    let editor_plugin = EditorPlugin::with_config(EditorConfig {
        enabled: true,
        unlock_cursor: true,
        draw_selection_outline: true,
        ..EditorConfig::default()
    });
    let mut runtime = VetraceRuntime::builder(editor_project)
        .mode(RuntimeMode::EditorPreview)
        .start_paused(true)
        .validate_project_files(false)
        .run_project_scripts(false)
        .stop_on_window_close(true)
        .add_plugin(studio_plugin)
        .add_plugin(editor_plugin)
        .build()?;

    runtime.start()?;
    runtime.run_until_stopped(max_frames, 1.0 / 60.0)?;
    Ok(())
}
