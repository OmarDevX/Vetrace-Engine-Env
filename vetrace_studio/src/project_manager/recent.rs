use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use vetrace_project::VetraceProject;

const RECENT_STORE_VERSION: u32 = 1;
const MAX_RECENT_PROJECTS: usize = 24;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct RecentStore {
    version: u32,
    projects: Vec<StoredRecentProject>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredRecentProject {
    path: PathBuf,
    last_opened_unix: u64,
}

#[derive(Clone, Debug, Default)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    pub version: String,
    pub engine_version: String,
    pub last_opened_unix: u64,
    pub available: bool,
    pub valid: bool,
    pub detail: String,
}

pub fn recent_projects() -> Vec<RecentProject> {
    load_store(&recent_store_path()).unwrap_or_default().into_entries()
}

pub fn record_recent_project(project: &VetraceProject) -> Result<(), String> {
    record_recent_path(project.root())
}

pub fn record_recent_path(path: &Path) -> Result<(), String> {
    let store_path = recent_store_path();
    let mut store = load_store(&store_path).unwrap_or_default();
    let path = normalize_path(path);
    store.projects.retain(|entry| normalize_path(&entry.path) != path);
    store.projects.insert(
        0,
        StoredRecentProject {
            path,
            last_opened_unix: unix_now(),
        },
    );
    store.projects.truncate(MAX_RECENT_PROJECTS);
    save_store(&store_path, &store)
}

pub fn remove_recent_path(path: &Path) -> Result<(), String> {
    let store_path = recent_store_path();
    let mut store = load_store(&store_path).unwrap_or_default();
    let normalized = normalize_path(path);
    store.projects.retain(|entry| normalize_path(&entry.path) != normalized);
    save_store(&store_path, &store)
}

pub fn recent_store_path() -> PathBuf {
    studio_config_directory().join("recent_projects.json")
}

pub fn studio_config_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(path) = std::env::var_os("APPDATA") {
            return PathBuf::from(path).join("Vetrace").join("Studio");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Vetrace")
                .join("Studio");
        }
    }
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(path).join("vetrace").join("studio");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("vetrace").join("studio");
    }
    std::env::temp_dir().join("vetrace-studio-config")
}

pub fn default_projects_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("VETRACE_PROJECTS_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "windows")]
    if let Some(home) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(home).join("Documents").join("VetraceProjects");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("VetraceProjects");
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

impl RecentStore {
    fn into_entries(mut self) -> Vec<RecentProject> {
        self.projects.sort_by(|left, right| right.last_opened_unix.cmp(&left.last_opened_unix));
        self.projects
            .into_iter()
            .map(|entry| inspect_recent(entry.path, entry.last_opened_unix))
            .collect()
    }
}

fn inspect_recent(path: PathBuf, last_opened_unix: u64) -> RecentProject {
    let available = path.is_dir();
    if !available {
        return RecentProject {
            name: path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Missing project")
                .to_string(),
            detail: "Project directory is missing".to_string(),
            path,
            last_opened_unix,
            available: false,
            valid: false,
            ..RecentProject::default()
        };
    }

    match VetraceProject::load(&path) {
        Ok(project) => {
            let report = project.validate_files();
            let valid = report.is_valid();
            let detail = if valid {
                let warnings = report.warning_count();
                if warnings == 0 { "Ready".to_string() } else { format!("Ready with {warnings} warning(s)") }
            } else {
                report.to_string()
            };
            RecentProject {
                path,
                name: project.manifest().project.name.clone(),
                version: project.manifest().project.version.clone(),
                engine_version: project.manifest().project.engine_version.clone(),
                last_opened_unix,
                available: true,
                valid,
                detail,
            }
        }
        Err(error) => {
            let unchecked = VetraceProject::load_unchecked(&path).ok();
            RecentProject {
                name: unchecked
                    .as_ref()
                    .map(|project| project.manifest().project.name.clone())
                    .or_else(|| path.file_name().and_then(|name| name.to_str()).map(str::to_owned))
                    .unwrap_or_else(|| "Invalid project".to_string()),
                version: unchecked
                    .as_ref()
                    .map(|project| project.manifest().project.version.clone())
                    .unwrap_or_default(),
                engine_version: unchecked
                    .as_ref()
                    .map(|project| project.manifest().project.engine_version.clone())
                    .unwrap_or_default(),
                detail: error.to_string(),
                path,
                last_opened_unix,
                available: true,
                valid: false,
            }
        }
    }
}

fn load_store(path: &Path) -> Result<RecentStore, String> {
    if !path.is_file() {
        return Ok(RecentStore {
            version: RECENT_STORE_VERSION,
            projects: Vec::new(),
        });
    }
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read recent projects '{}': {error}", path.display()))?;
    let mut store: RecentStore = serde_json::from_str(&source)
        .map_err(|error| format!("failed to parse recent projects '{}': {error}", path.display()))?;
    if store.version == 0 {
        store.version = RECENT_STORE_VERSION;
    }
    Ok(store)
}

fn save_store(path: &Path, store: &RecentStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create Studio config '{}': {error}", parent.display()))?;
    }
    let mut store = store.clone();
    store.version = RECENT_STORE_VERSION;
    let source = serde_json::to_vec_pretty(&store)
        .map_err(|error| format!("failed to serialize recent projects: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    let mut file = fs::File::create(&temporary)
        .map_err(|error| format!("failed to create '{}': {error}", temporary.display()))?;
    file.write_all(&source)
        .map_err(|error| format!("failed to write '{}': {error}", temporary.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to flush '{}': {error}", temporary.display()))?;
    drop(file);
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to replace '{}': {error}", path.display()))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("failed to replace '{}': {error}", path.display()))
}

fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_store_round_trips_and_deduplicates() {
        let root = std::env::temp_dir().join(format!("vetrace-recent-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let store_path = root.join("recent.json");
        let project = root.join("Project");
        fs::create_dir_all(&project).unwrap();

        let mut store = RecentStore {
            version: RECENT_STORE_VERSION,
            projects: vec![
                StoredRecentProject { path: project.clone(), last_opened_unix: 1 },
                StoredRecentProject { path: project.clone(), last_opened_unix: 2 },
            ],
        };
        let normalized = normalize_path(&project);
        store.projects.retain(|entry| normalize_path(&entry.path) != normalized);
        store.projects.insert(0, StoredRecentProject { path: normalized, last_opened_unix: 3 });
        save_store(&store_path, &store).unwrap();
        let loaded = load_store(&store_path).unwrap();
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.projects[0].last_opened_unix, 3);

        let _ = fs::remove_dir_all(root);
    }
}
