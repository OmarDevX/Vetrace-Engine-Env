use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vetrace_project::VetraceProject;
use vetrace_scripting_lua::{LuaDebuggerCommand, LuaDebuggerEvent, LuaPausedState};

#[derive(Clone, Debug, Default)]
pub struct DebuggerSnapshot {
    pub connected: bool,
    pub paused: Option<LuaPausedState>,
    pub breakpoints: BTreeMap<String, BTreeSet<usize>>,
    pub watches: Vec<String>,
    pub break_on_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedDebuggerState {
    #[serde(default)]
    breakpoints: BTreeMap<String, BTreeSet<usize>>,
    #[serde(default)]
    watches: Vec<String>,
    #[serde(default = "default_true")]
    break_on_error: bool,
}

impl Default for PersistedDebuggerState {
    fn default() -> Self {
        Self {
            breakpoints: BTreeMap::new(),
            watches: Vec::new(),
            break_on_error: true,
        }
    }
}

fn default_true() -> bool { true }

pub struct StudioDebugger {
    path: PathBuf,
    persisted: PersistedDebuggerState,
    connected: bool,
    paused: Option<LuaPausedState>,
}

impl StudioDebugger {
    pub fn load(project: &VetraceProject) -> Self {
        let path = project.paths().editor().join("debugger.json");
        let persisted = std::fs::read_to_string(&path)
            .ok()
            .and_then(|source| serde_json::from_str(&source).ok())
            .unwrap_or_default();
        Self { path, persisted, connected: false, paused: None }
    }

    pub fn snapshot(&self) -> DebuggerSnapshot {
        DebuggerSnapshot {
            connected: self.connected,
            paused: self.paused.clone(),
            breakpoints: self.persisted.breakpoints.clone(),
            watches: self.persisted.watches.clone(),
            break_on_error: self.persisted.break_on_error,
        }
    }

    pub fn reset_connection(&mut self) {
        self.connected = false;
        self.paused = None;
    }

    pub fn handle_event(&mut self, event: &LuaDebuggerEvent) {
        match event {
            LuaDebuggerEvent::Ready => {
                self.connected = true;
                self.paused = None;
            }
            LuaDebuggerEvent::Paused { state } => {
                self.connected = true;
                self.paused = Some(state.clone());
            }
            LuaDebuggerEvent::Resumed => {
                self.connected = true;
                self.paused = None;
            }
            LuaDebuggerEvent::Error { .. } => self.connected = true,
        }
    }

    pub fn toggle_breakpoint(&mut self, path: &Path, line: usize, project: &VetraceProject) -> Result<(), String> {
        let project_path = project
            .paths()
            .to_project_path(path)
            .map_err(|error| error.to_string())?
            .as_str()
            .replace('\\', "/");
        let lines = self.persisted.breakpoints.entry(project_path.clone()).or_default();
        if !lines.insert(line.max(1)) { lines.remove(&line.max(1)); }
        if lines.is_empty() { self.persisted.breakpoints.remove(&project_path); }
        self.save()
    }

    pub fn set_watches(&mut self, watches: Vec<String>) -> Result<(), String> {
        self.persisted.watches = watches
            .into_iter()
            .map(|watch| watch.trim().to_owned())
            .filter(|watch| !watch.is_empty())
            .collect();
        self.save()
    }

    pub fn set_break_on_error(&mut self, enabled: bool) -> Result<(), String> {
        self.persisted.break_on_error = enabled;
        self.save()
    }

    pub fn configuration_commands(&self) -> [LuaDebuggerCommand; 3] {
        [
            LuaDebuggerCommand::SetBreakpoints { breakpoints: self.persisted.breakpoints.clone() },
            LuaDebuggerCommand::SetWatches { expressions: self.persisted.watches.clone() },
            LuaDebuggerCommand::SetBreakOnError { enabled: self.persisted.break_on_error },
        ]
    }

    pub fn breakpoint_command(&self) -> LuaDebuggerCommand {
        LuaDebuggerCommand::SetBreakpoints { breakpoints: self.persisted.breakpoints.clone() }
    }

    pub fn watches_command(&self) -> LuaDebuggerCommand {
        LuaDebuggerCommand::SetWatches { expressions: self.persisted.watches.clone() }
    }

    pub fn break_on_error_command(&self) -> LuaDebuggerCommand {
        LuaDebuggerCommand::SetBreakOnError { enabled: self.persisted.break_on_error }
    }

    fn save(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let source = serde_json::to_vec_pretty(&self.persisted).map_err(|error| error.to_string())?;
        let temporary = self.path.with_extension("json.tmp");
        std::fs::write(&temporary, source).map_err(|error| error.to_string())?;
        #[cfg(windows)]
        if self.path.exists() {
            std::fs::remove_file(&self.path).map_err(|error| error.to_string())?;
        }
        std::fs::rename(&temporary, &self.path).map_err(|error| error.to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use vetrace_project::ProjectManifest;

    #[test]
    fn debugger_configuration_persists() {
        let root = std::env::temp_dir().join(format!(
            "vetrace-studio-debugger-{}",
            uuid::Uuid::new_v4()
        ));
        let project = VetraceProject::create(
            &root,
            ProjectManifest::new("Debugger Test", env!("CARGO_PKG_VERSION")),
        ).unwrap();
        let script = project.root().join("assets/scripts/player.lua");
        std::fs::write(&script, "return {}").unwrap();

        let mut debugger = StudioDebugger::load(&project);
        debugger.toggle_breakpoint(&script, 12, &project).unwrap();
        debugger.set_watches(vec!["self.health".into(), " speed ".into()]).unwrap();
        debugger.set_break_on_error(false).unwrap();

        let loaded = StudioDebugger::load(&project).snapshot();
        assert!(loaded.breakpoints.values().any(|lines| lines.contains(&12)));
        assert_eq!(loaded.watches, vec!["self.health", "speed"]);
        assert!(!loaded.break_on_error);
        let _ = std::fs::remove_dir_all(root);
    }
}
