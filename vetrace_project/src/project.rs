use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{
    find_project_root, validate_manifest, validate_project_files, ProjectError, ProjectManifest, ProjectPath,
    ProjectPaths, ProjectResult, ValidationReport, PROJECT_MANIFEST_FILE,
};

#[derive(Clone, Debug)]
pub struct VetraceProject {
    manifest: ProjectManifest,
    paths: ProjectPaths,
}

impl VetraceProject {
    pub fn create(root: impl AsRef<Path>, manifest: ProjectManifest) -> ProjectResult<Self> {
        let paths = ProjectPaths::new(root)?;
        if paths.manifest().exists() {
            return Err(ProjectError::ManifestAlreadyExists(paths.manifest().to_path_buf()));
        }
        validate_manifest(&manifest).into_result()?;
        paths.ensure_layout()?;

        let project = Self { manifest, paths };
        project.save()?;
        Ok(project)
    }

    pub fn create_new(
        root: impl AsRef<Path>,
        name: impl Into<String>,
        engine_version: impl Into<String>,
    ) -> ProjectResult<Self> {
        Self::create(root, ProjectManifest::new(name, engine_version))
    }

    /// Opens a manifest path or a project root. This does not walk parent
    /// directories; use [`Self::discover`] for that behavior.
    pub fn load(path: impl AsRef<Path>) -> ProjectResult<Self> {
        let project = Self::load_unchecked(path)?;
        project.validate_manifest().into_result()?;
        Ok(project)
    }

    /// Parses a project without applying semantic validation. Unsafe paths are
    /// still rejected by `ProjectPath` deserialization. This is intended for
    /// editor repair and migration tools that need to open a malformed project
    /// and present its full validation report.
    pub fn load_unchecked(path: impl AsRef<Path>) -> ProjectResult<Self> {
        let path = path.as_ref();
        let manifest_path = if path.file_name().and_then(|name| name.to_str()) == Some(PROJECT_MANIFEST_FILE) {
            path.to_path_buf()
        } else {
            path.join(PROJECT_MANIFEST_FILE)
        };
        if !manifest_path.is_file() {
            return Err(ProjectError::ManifestNotFound {
                start: path.to_path_buf(),
                file_name: PROJECT_MANIFEST_FILE,
            });
        }
        let root = manifest_path.parent().unwrap_or(Path::new("."));
        let source = fs::read_to_string(&manifest_path)
            .map_err(|error| ProjectError::io("read project manifest", &manifest_path, error))?;
        let manifest = toml::from_str(&source).map_err(|source| ProjectError::ParseManifest {
            path: manifest_path.clone(),
            source,
        })?;
        Ok(Self {
            manifest,
            paths: ProjectPaths::new(root)?,
        })
    }

    pub fn discover(start: impl AsRef<Path>) -> ProjectResult<Self> {
        Self::load(find_project_root(start)?)
    }

    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    pub fn manifest_mut(&mut self) -> &mut ProjectManifest {
        &mut self.manifest
    }

    pub fn paths(&self) -> &ProjectPaths {
        &self.paths
    }

    pub fn root(&self) -> &Path {
        self.paths.root()
    }

    pub fn main_scene_path(&self) -> PathBuf {
        self.paths.resolve(&self.manifest.runtime.main_scene)
    }

    pub fn resolve(&self, path: &ProjectPath) -> PathBuf {
        self.paths.resolve(path)
    }

    pub fn validate_manifest(&self) -> ValidationReport {
        validate_manifest(&self.manifest)
    }

    pub fn validate_files(&self) -> ValidationReport {
        validate_project_files(&self.manifest, &self.paths)
    }

    pub fn save(&self) -> ProjectResult<()> {
        self.validate_manifest().into_result()?;
        self.paths.ensure_layout()?;
        let source = toml::to_string_pretty(&self.manifest).map_err(ProjectError::SerializeManifest)?;
        write_atomic(self.paths.manifest(), source.as_bytes())
    }

    pub fn reload(&mut self) -> ProjectResult<()> {
        let loaded = Self::load(self.paths.root())?;
        self.manifest = loaded.manifest;
        Ok(())
    }
}

fn write_atomic(path: &Path, contents: &[u8]) -> ProjectResult<()> {
    let temporary = path.with_file_name(format!(".{PROJECT_MANIFEST_FILE}.tmp"));
    let mut file = fs::File::create(&temporary)
        .map_err(|error| ProjectError::io("create temporary project manifest", &temporary, error))?;
    file.write_all(contents)
        .map_err(|error| ProjectError::io("write temporary project manifest", &temporary, error))?;
    file.sync_all()
        .map_err(|error| ProjectError::io("flush temporary project manifest", &temporary, error))?;
    drop(file);

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| ProjectError::io("replace project manifest", path, error))?;
    }

    fs::rename(&temporary, path)
        .map_err(|error| ProjectError::io("replace project manifest", path, error))?;
    Ok(())
}
