use super::*;

pub(super) fn update_project_script_references(
    project_root: &Path,
    old_relative: &str,
    new_relative: &str,
    workspace: &mut ScriptWorkspace,
) -> Result<(), String> {
    let mut files = vec![project_root.join("project.vetrace.toml")];
    collect_reference_files(&project_root.join("assets"), &mut files)?;
    for path in files {
        if !path.is_file() { continue; }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
        if !text.contains(old_relative) { continue; }
        let updated = text.replace(old_relative, new_relative);
        let temporary = path.with_extension(format!("{}.tmp", path.extension().and_then(|value| value.to_str()).unwrap_or("file")));
        std::fs::write(&temporary, &updated)
            .map_err(|error| format!("failed to write '{}': {error}", temporary.display()))?;
        std::fs::rename(&temporary, &path)
            .map_err(|error| format!("failed to replace '{}': {error}", path.display()))?;
        if let Some(document) = workspace.documents_mut().iter_mut().find(|document| document.path == path) {
            if document.is_dirty() {
                document.set_text(document.text.replace(old_relative, new_relative));
            } else {
                document.replace_from_disk(updated);
            }
        }
    }
    Ok(())
}

pub(super) fn collect_reference_files(directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    if !directory.is_dir() { return Ok(()); }
    for entry in std::fs::read_dir(directory)
        .map_err(|error| format!("failed to inspect '{}': {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_reference_files(&path, output)?;
        } else if matches!(path.extension().and_then(|value| value.to_str()), Some("lua" | "vscene" | "vprefab" | "vmat")) {
            output.push(path);
        }
    }
    Ok(())
}
