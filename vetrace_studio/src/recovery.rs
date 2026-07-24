use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vetrace_project::{ProjectPath, VetraceProject};
use vetrace_scene::SceneDocument;

pub const RECOVERY_FORMAT_VERSION: u32 = 1;
const AUTOSAVE_INTERVAL_SECONDS: f32 = 15.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryScript {
    pub path: ProjectPath,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryBundle {
    pub format_version: u32,
    pub project_id: Uuid,
    pub saved_unix_ms: u128,
    pub scene_path: ProjectPath,
    pub scene: SceneDocument,
    pub scripts: Vec<RecoveryScript>,
}

pub struct RecoveryManager {
    path: PathBuf,
    elapsed: f32,
    available: Option<RecoveryBundle>,
}

impl RecoveryManager {
    pub fn new(project: &VetraceProject) -> Self {
        let path = project.paths().editor().join("recovery.json");
        let available = load_bundle(&path)
            .ok()
            .flatten()
            .filter(|bundle| bundle.project_id == project.manifest().project.id)
            .filter(|bundle| bundle.format_version == RECOVERY_FORMAT_VERSION);
        Self { path, elapsed: 0.0, available }
    }

    pub fn is_available(&self) -> bool { self.available.is_some() }
    pub fn bundle(&self) -> Option<&RecoveryBundle> { self.available.as_ref() }
    pub fn take(&mut self) -> Option<RecoveryBundle> { self.available.take() }

    pub fn tick(&mut self, dt: f32) -> bool {
        self.elapsed += dt.max(0.0);
        if self.elapsed >= AUTOSAVE_INTERVAL_SECONDS {
            self.elapsed = 0.0;
            true
        } else {
            false
        }
    }

    pub fn save(
        &mut self,
        project: &VetraceProject,
        scene_path: ProjectPath,
        scene: SceneDocument,
        scripts: Vec<RecoveryScript>,
    ) -> Result<(), String> {
        let bundle = RecoveryBundle {
            format_version: RECOVERY_FORMAT_VERSION,
            project_id: project.manifest().project.id,
            saved_unix_ms: now_unix_ms(),
            scene_path,
            scene,
            scripts,
        };
        let bytes = serde_json::to_vec_pretty(&bundle)
            .map_err(|error| format!("failed to serialize recovery data: {error}"))?;
        write_atomic(&self.path, &bytes)?;
        self.available = Some(bundle);
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), String> {
        self.available = None;
        self.elapsed = 0.0;
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("failed to remove '{}': {error}", self.path.display())),
        }
    }
}

fn load_bundle(path: &Path) -> Result<Option<RecoveryBundle>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("failed to read '{}': {error}", path.display())),
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("failed to parse '{}': {error}", path.display()))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "recovery path has no parent".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)
        .map_err(|error| format!("failed to write '{}': {error}", temporary.display()))?;
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to replace '{}': {error}", path.display()))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| format!("failed to replace '{}': {error}", path.display()))
}

fn now_unix_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}


#[cfg(test)]
mod tests {
    use super::*;
    use vetrace_project::ProjectManifest;

    fn project(label: &str) -> (PathBuf, VetraceProject) {
        let root = std::env::temp_dir().join(format!(
            "vetrace-recovery-{label}-{}",
            Uuid::new_v4()
        ));
        let project = VetraceProject::create(
            &root,
            ProjectManifest::new("Recovery Test", env!("CARGO_PKG_VERSION")),
        ).expect("create project");
        (root, project)
    }

    #[test]
    fn recovery_round_trip_and_clear() {
        let (root, project) = project("round-trip");
        let mut manager = RecoveryManager::new(&project);
        assert!(!manager.is_available());
        manager.save(
            &project,
            ProjectPath::new("assets/scenes/unsaved.vscene").unwrap(),
            SceneDocument::new("Unsaved"),
            vec![RecoveryScript {
                path: ProjectPath::new("assets/scripts/player.lua").unwrap(),
                text: "return {}".to_owned(),
            }],
        ).unwrap();
        assert!(manager.is_available());

        let loaded = RecoveryManager::new(&project);
        let bundle = loaded.bundle().expect("persisted recovery");
        assert_eq!(bundle.scene.name, "Unsaved");
        assert_eq!(bundle.scripts.len(), 1);

        manager.clear().unwrap();
        assert!(!RecoveryManager::new(&project).is_available());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn autosave_tick_is_interval_based() {
        let (_root, project) = project("tick");
        let mut manager = RecoveryManager::new(&project);
        assert!(!manager.tick(AUTOSAVE_INTERVAL_SECONDS - 0.1));
        assert!(manager.tick(0.1));
        assert!(!manager.tick(0.0));
        let _ = fs::remove_dir_all(project.root());
    }
}
