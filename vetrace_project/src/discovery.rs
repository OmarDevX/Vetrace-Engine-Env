use std::fs;
use std::path::{Path, PathBuf};

use crate::{ProjectError, ProjectResult, PROJECT_MANIFEST_FILE};

pub fn find_project_root(start: impl AsRef<Path>) -> ProjectResult<PathBuf> {
    let start = start.as_ref();
    let mut current = if start.is_file() {
        start.parent().unwrap_or(start).to_path_buf()
    } else {
        start.to_path_buf()
    };

    if current.is_relative() {
        let cwd = std::env::current_dir()
            .map_err(|error| ProjectError::io("read current directory", ".", error))?;
        current = cwd.join(current);
    }

    loop {
        if current.join(PROJECT_MANIFEST_FILE).is_file() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }

    Err(ProjectError::ManifestNotFound {
        start: start.to_path_buf(),
        file_name: PROJECT_MANIFEST_FILE,
    })
}

/// Recursively discovers Vetrace project roots without following directory
/// symlinks. Heavy generated directories are skipped.
pub fn discover_projects(root: impl AsRef<Path>, max_depth: usize) -> ProjectResult<Vec<PathBuf>> {
    let root = root.as_ref();
    let mut projects = Vec::new();
    visit(root, 0, max_depth, &mut projects)?;
    projects.sort();
    projects.dedup();
    Ok(projects)
}

fn visit(path: &Path, depth: usize, max_depth: usize, projects: &mut Vec<PathBuf>) -> ProjectResult<()> {
    if path.join(PROJECT_MANIFEST_FILE).is_file() {
        projects.push(path.to_path_buf());
        return Ok(());
    }
    if depth >= max_depth {
        return Ok(());
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
        Err(error) => return Err(ProjectError::io("read directory", path, error)),
    };

    for entry in entries {
        let entry = entry.map_err(|error| ProjectError::io("read directory entry", path, error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| ProjectError::io("inspect directory entry", entry.path(), error))?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(name.as_ref(), ".git" | ".vetrace" | "target" | "node_modules") {
            continue;
        }
        visit(&entry.path(), depth + 1, max_depth, projects)?;
    }
    Ok(())
}
