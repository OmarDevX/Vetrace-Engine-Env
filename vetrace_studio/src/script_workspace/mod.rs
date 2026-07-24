use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use vetrace_lua_tools::LuaLanguageService;
use vetrace_project::{ProjectPath, VetraceProject};

use crate::recovery::RecoveryScript;
use vetrace_script_editor::{
    DiagnosticSeverity, ExternalChange, ExternalChangeResolution, LanguageRegistry,
    ScriptDiagnostic, ScriptWorkspace, TextPosition, TextRange, line_range,
};

mod references;
mod runtime_diagnostics;
mod session;

use references::update_project_script_references;
pub use runtime_diagnostics::parse_console_script_location;
use runtime_diagnostics::parse_runtime_diagnostic;
use session::{load_script_session, save_script_session};



#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScriptViewState {
    pub cursor_byte: usize,
    pub line: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ScriptSessionFile {
    #[serde(default = "script_session_version")]
    version: u32,
    #[serde(default)]
    open_documents: Vec<String>,
    #[serde(default)]
    active_document: Option<String>,
    #[serde(default)]
    views: BTreeMap<String, ScriptViewState>,
}

const fn script_session_version() -> u32 { 1 }

impl Default for ScriptSessionFile {
    fn default() -> Self {
        Self {
            version: script_session_version(),
            open_documents: Vec::new(),
            active_document: None,
            views: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeScriptDiagnostic {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

pub struct StudioScriptState {
    pub workspace: ScriptWorkspace,
    pub focus_requested: bool,
    pub target_line: Option<usize>,
    pub runtime_diagnostics: Vec<RuntimeScriptDiagnostic>,
    pub external_changes: Vec<ExternalChange>,
    pub view_states: BTreeMap<PathBuf, ScriptViewState>,
}

#[derive(Clone)]
pub struct StudioScripts {
    project_root: PathBuf,
    session_path: PathBuf,
    inner: Arc<Mutex<StudioScriptState>>,
    last_maintenance: Arc<Mutex<Instant>>,
}

#[derive(Clone, Debug)]
pub struct ScriptSaveResult {
    pub path: PathBuf,
    pub error_count: usize,
}

impl StudioScripts {
    pub fn new(project: &VetraceProject) -> Self {
        let mut registry = LanguageRegistry::new();
        registry.register(LuaLanguageService);
        let project_root = project.root().to_path_buf();
        let session_path = project.paths().editor().join("script-session.json");
        let session = load_script_session(&session_path).unwrap_or_default();
        let mut workspace = ScriptWorkspace::new(registry);
        for relative in &session.open_documents {
            let path = project_root.join(relative);
            if path.is_file() { let _ = workspace.open(path); }
        }
        if let Some(active) = &session.active_document {
            let active = project_root.join(active);
            if let Some(index) = workspace.documents().iter().position(|document| document.path == active) {
                workspace.set_active(index);
            }
        }
        let view_states = session.views.into_iter()
            .map(|(path, view)| (project_root.join(path), view))
            .collect();
        Self {
            project_root,
            session_path,
            inner: Arc::new(Mutex::new(StudioScriptState {
                workspace,
                focus_requested: false,
                target_line: None,
                runtime_diagnostics: Vec::new(),
                external_changes: Vec::new(),
                view_states,
            })),
            last_maintenance: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(2))),
        }
    }

    pub fn with_state<R>(&self, operation: impl FnOnce(&mut StudioScriptState) -> R) -> Option<R> {
        self.inner.lock().ok().map(|mut state| operation(&mut state))
    }

    pub fn maintenance(&self) {
        let Ok(mut last) = self.last_maintenance.lock() else { return; };
        if last.elapsed() < Duration::from_secs(1) { return; }
        *last = Instant::now();
        if let Ok(mut state) = self.inner.lock() {
            let changes = state.workspace.poll_external_changes();
            for change in changes {
                if !change.has_local_changes && change.disk_text.is_some() {
                    let _ = state.workspace.resolve_external_change(&change, ExternalChangeResolution::Reload);
                    let _ = state.workspace.analyze(change.document_index);
                } else if !state.external_changes.iter().any(|existing| existing.path == change.path) {
                    state.external_changes.push(change);
                }
            }
            let open_paths = state.workspace.documents().iter()
                .map(|document| document.path.clone())
                .collect::<Vec<_>>();
            state.external_changes.retain(|change| open_paths.iter().any(|path| path == &change.path));
            let _ = save_script_session(&self.project_root, &self.session_path, &state);
        }
    }

    pub fn resolve_external_change(
        &self,
        path: &Path,
        resolution: ExternalChangeResolution,
    ) -> Result<(), String> {
        let mut state = self.inner.lock().map_err(|_| "script editor state is unavailable".to_string())?;
        let index = state.external_changes.iter().position(|change| change.path == path)
            .ok_or_else(|| "external change no longer exists".to_string())?;
        let change = state.external_changes.remove(index);
        state.workspace.resolve_external_change(&change, resolution).map_err(|error| error.to_string())?;
        if change.document_index < state.workspace.documents().len() {
            let _ = state.workspace.analyze(change.document_index);
        }
        save_script_session(&self.project_root, &self.session_path, &state).map_err(|error| error.to_string())
    }

    pub fn rename_script(&self, index: usize, relative_path: &str) -> Result<PathBuf, String> {
        let target = ProjectPath::new(relative_path).map_err(|error| error.to_string())?;
        if !target.starts_with("assets/scripts") || target.extension() != Some("lua") {
            return Err("scripts must remain under assets/scripts/ and use the .lua extension".into());
        }
        let new_path = self.project_root.join(target.as_str());
        let mut state = self.inner.lock().map_err(|_| "script editor state is unavailable".to_string())?;
        let old_path = state.workspace.documents().get(index)
            .ok_or_else(|| "script tab no longer exists".to_string())?
            .path.clone();
        let old_relative = old_path.strip_prefix(&self.project_root)
            .map_err(|_| "script path is outside the project".to_string())?
            .to_string_lossy().replace('\\', "/");
        let new_relative = target.as_str().replace('\\', "/");
        let renamed = state.workspace.rename_document(index, &new_path).map_err(|error| error.to_string())?;
        update_project_script_references(&self.project_root, &old_relative, &new_relative, &mut state.workspace)?;
        if let Some(view) = state.view_states.remove(&old_path) { state.view_states.insert(renamed.clone(), view); }
        save_script_session(&self.project_root, &self.session_path, &state).map_err(|error| error.to_string())?;
        Ok(renamed)
    }

    pub fn delete_script(&self, index: usize, discard: bool) -> Result<PathBuf, String> {
        let mut state = self.inner.lock().map_err(|_| "script editor state is unavailable".to_string())?;
        let path = state.workspace.delete_document(index, discard).map_err(|error| error.to_string())?;
        state.view_states.remove(&path);
        state.external_changes.retain(|change| change.path != path);
        save_script_session(&self.project_root, &self.session_path, &state).map_err(|error| error.to_string())?;
        Ok(path)
    }

    pub fn open(&self, path: impl AsRef<Path>, line: Option<usize>) -> Result<usize, String> {
        let path = self.resolve_script_path(path.as_ref())?;
        let mut state = self.inner.lock().map_err(|_| "script editor state is unavailable".to_string())?;
        let index = state.workspace.open(&path).map_err(|error| error.to_string())?;
        state.focus_requested = true;
        state.target_line = line;
        let _ = save_script_session(&self.project_root, &self.session_path, &state);
        Ok(index)
    }

    pub fn save(&self, index: usize) -> Result<ScriptSaveResult, String> {
        let mut state = self.inner.lock().map_err(|_| "script editor state is unavailable".to_string())?;
        state.workspace.analyze(index).map_err(|error| error.to_string())?;
        let error_count = state.workspace.documents()[index]
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            .count();
        let path = state.workspace.save(index).map_err(|error| error.to_string())?;
        state.runtime_diagnostics.retain(|diagnostic| diagnostic.path != path);
        Ok(ScriptSaveResult { path, error_count })
    }

    pub fn save_all(&self) -> Result<Vec<ScriptSaveResult>, String> {
        let count = self.inner.lock()
            .map_err(|_| "script editor state is unavailable".to_string())?
            .workspace
            .documents()
            .len();
        let mut results = Vec::new();
        for index in 0..count {
            if self.with_state(|state| state.workspace.documents()[index].is_dirty()).unwrap_or(false) {
                results.push(self.save(index)?);
            }
        }
        Ok(results)
    }

    pub fn has_dirty_documents(&self) -> bool {
        self.inner.lock().map(|state| {
            state.workspace.documents().iter().any(|document| document.is_dirty())
        }).unwrap_or(false)
    }

    pub fn recovery_scripts(&self, project: &VetraceProject) -> Vec<RecoveryScript> {
        self.inner.lock().map(|state| {
            state.workspace.documents().iter()
                .filter(|document| document.is_dirty())
                .filter_map(|document| {
                    let path = project.paths().to_project_path(&document.path).ok()?;
                    Some(RecoveryScript { path, text: document.text.clone() })
                })
                .collect()
        }).unwrap_or_default()
    }

    pub fn restore_recovery_scripts(&self, project: &VetraceProject, scripts: &[RecoveryScript]) -> Vec<String> {
        let mut messages = Vec::new();
        for recovered in scripts {
            let path = project.paths().resolve(&recovered.path);
            match self.open(&path, None) {
                Ok(index) => {
                    let restored = self.with_state(|state| {
                        let Some(document) = state.workspace.documents_mut().get_mut(index) else { return false; };
                        document.set_text(recovered.text.clone());
                        true
                    }).unwrap_or(false);
                    if restored { messages.push(format!("Recovered {}", recovered.path)); }
                }
                Err(error) => messages.push(format!("Could not recover {}: {error}", recovered.path)),
            }
        }
        messages
    }

    pub fn take_focus_request(&self) -> bool {
        self.with_state(|state| std::mem::take(&mut state.focus_requested)).unwrap_or(false)
    }

    pub fn ingest_player_output(&self, text: &str) {
        let Some(parsed) = parse_runtime_diagnostic(&self.project_root, text) else { return; };
        let _ = self.with_state(|state| {
            if !state.runtime_diagnostics.iter().any(|diagnostic| diagnostic == &parsed) {
                state.runtime_diagnostics.push(parsed);
                if state.runtime_diagnostics.len() > 256 {
                    let excess = state.runtime_diagnostics.len() - 256;
                    state.runtime_diagnostics.drain(..excess);
                }
            }
        });
    }

    pub fn clear_runtime_diagnostics(&self, path: &Path) {
        let path = path.to_path_buf();
        let _ = self.with_state(|state| {
            state.runtime_diagnostics.retain(|diagnostic| diagnostic.path != path);
        });
    }

    pub fn syntax_and_runtime_diagnostics(
        state: &StudioScriptState,
        document_index: usize,
    ) -> Vec<ScriptDiagnostic> {
        let Some(document) = state.workspace.documents().get(document_index) else { return Vec::new(); };
        let mut diagnostics = document.diagnostics.clone();
        for runtime in state.runtime_diagnostics.iter().filter(|runtime| runtime.path == document.path) {
            diagnostics.push(ScriptDiagnostic {
                severity: DiagnosticSeverity::Error,
                message: runtime.message.clone(),
                range: line_range(&document.text, runtime.line),
                position: TextPosition::new(runtime.line, runtime.column),
                code: Some("lua.runtime".into()),
                actions: Vec::new(),
            });
        }
        diagnostics.sort_by_key(|diagnostic| (diagnostic.position.line, diagnostic.position.column));
        diagnostics
    }

    fn resolve_script_path(&self, path: &Path) -> Result<PathBuf, String> {
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };
        let canonical_root = std::fs::canonicalize(&self.project_root)
            .map_err(|error| format!("failed to resolve project root: {error}"))?;
        let canonical = std::fs::canonicalize(&candidate)
            .map_err(|error| format!("failed to resolve '{}': {error}", candidate.display()))?;
        if !canonical.starts_with(&canonical_root) {
            return Err("script path is outside the project".into());
        }
        if canonical
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("lua"))
            != Some(true)
        {
            return Err("the built-in script editor currently opens .lua files".into());
        }
        let project_path = ProjectPath::new(
            canonical.strip_prefix(&canonical_root)
                .map_err(|_| "script path is outside the project".to_string())?
                .to_string_lossy(),
        ).map_err(|error| error.to_string())?;
        if !project_path.starts_with("assets/scripts") {
            return Err("gameplay scripts must be under assets/scripts/".into());
        }
        Ok(canonical)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_runtime_diagnostics() {
        let root = Path::new("/game");
        let parsed = parse_runtime_diagnostic(
            root,
            "VETRACE_SCRIPT_DIAGNOSTIC\tassets/scripts/player.lua\t27\t4\tbad value",
        ).unwrap();
        assert_eq!(parsed.path, root.join("assets/scripts/player.lua"));
        assert_eq!(parsed.line, 27);
        assert_eq!(parsed.column, 4);
    }

    #[test]
    fn parses_normal_lua_trace_locations() {
        let root = Path::new("/game");
        let parsed = parse_runtime_diagnostic(
            root,
            "Lua update error: [string \"assets/scripts/player.lua\"]:18: boom",
        ).unwrap();
        assert_eq!(parsed.path, root.join("assets/scripts/player.lua"));
        assert_eq!(parsed.line, 18);
    }
}
