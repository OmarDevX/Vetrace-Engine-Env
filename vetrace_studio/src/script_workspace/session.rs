use super::*;

pub(super) fn load_script_session(path: &Path) -> Result<ScriptSessionFile, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(ScriptSessionFile::default()),
        Err(error) => return Err(format!("failed to read '{}': {error}", path.display())),
    };
    serde_json::from_str(&text).map_err(|error| format!("failed to parse '{}': {error}", path.display()))
}

pub(super) fn save_script_session(
    project_root: &Path,
    path: &Path,
    state: &StudioScriptState,
) -> Result<(), std::io::Error> {
    let relative = |absolute: &Path| absolute.strip_prefix(project_root).ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"));
    let open_documents = state.workspace.documents().iter()
        .filter_map(|document| relative(&document.path))
        .collect::<Vec<_>>();
    let active_document = state.workspace.active()
        .and_then(|document| relative(&document.path));
    let views = state.view_states.iter()
        .filter_map(|(path, view)| relative(path).map(|path| (path, view.clone())))
        .collect();
    let session = ScriptSessionFile {
        version: script_session_version(),
        open_documents,
        active_document,
        views,
    };
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&session).unwrap_or_default())?;
    std::fs::rename(temporary, path)
}
