use std::path::PathBuf;

use vetrace_project::{ProjectPath, VetraceProject};

/// Project sandbox made available to trusted gameplay scripts.
///
/// Every Lua asset/scene operation must pass through `ProjectPath`, then through
/// canonical project-root checks. Lua never receives an unrestricted filesystem
/// handle or an arbitrary host path.
#[derive(Clone, Debug)]
pub struct LuaProjectContext {
    project: VetraceProject,
}

impl LuaProjectContext {
    pub fn new(project: VetraceProject) -> Self { Self { project } }

    pub fn project(&self) -> &VetraceProject { &self.project }

    pub fn resolve_existing(&self, raw: &str) -> Result<PathBuf, String> {
        let path = ProjectPath::new(raw).map_err(|error| error.to_string())?;
        if !path.starts_with("assets") {
            return Err(format!("Lua project access is limited to assets/: {path}"));
        }
        self.project
            .paths()
            .resolve_existing(&path)
            .map_err(|error| error.to_string())
    }

    pub fn exists(&self, raw: &str) -> bool {
        self.resolve_existing(raw).is_ok()
    }
}
