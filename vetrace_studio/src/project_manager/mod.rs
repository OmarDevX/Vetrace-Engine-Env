mod bridge;
mod plugin;
mod recent;
mod template;
mod ui;

use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};

pub use bridge::{ProjectManagerBridge, ProjectManagerCommand, ProjectManagerSnapshot};
pub use plugin::ProjectManagerPlugin;
pub use recent::{
    default_projects_directory, recent_projects, record_recent_path, remove_recent_path,
    RecentProject,
};
pub use template::{
    create_project, create_temporary_manager_project, slugify_project_name, CreateProjectRequest,
    ProjectTemplate,
};
pub use ui::ProjectManagerEguiTool;

use vetrace_project::VetraceProject;
use vetrace_runtime::{RuntimeMode, VetraceRuntime};

pub fn run_project_manager(max_frames: Option<usize>) -> Result<(), Box<dyn Error>> {
    let temporary = create_temporary_manager_project()?;
    let mut runtime = VetraceRuntime::builder(temporary.project.clone())
        .mode(RuntimeMode::EditorPreview)
        .start_paused(true)
        .validate_project_files(true)
        .run_project_scripts(false)
        .stop_on_window_close(true)
        .add_plugin(ProjectManagerPlugin::new())
        .build()?;
    runtime.start()?;
    runtime.run_until_stopped(max_frames, 1.0 / 60.0)?;
    drop(runtime);
    drop(temporary);
    Ok(())
}

pub fn record_recent_project(project: &VetraceProject) -> Result<(), String> {
    recent::record_recent_project(project)
}

pub fn launch_studio_project(project_root: &Path) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to locate Vetrace Studio executable: {error}"))?;
    Command::new(executable)
        .arg("--project")
        .arg(project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open project in Vetrace Studio: {error}"))
}

pub fn launch_project_manager() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to locate Vetrace Studio executable: {error}"))?;
    Command::new(executable)
        .arg("--project-manager")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to open the Vetrace project manager: {error}"))
}
