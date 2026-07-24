use std::borrow::Borrow;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{ProjectError, ProjectResult, PROJECT_MANIFEST_FILE};

/// A normalized, UTF-8, project-relative path.
///
/// `ProjectPath` rejects absolute paths, parent traversal, Windows drive paths,
/// UNC paths, and empty paths at deserialization time. Separators are stored as
/// `/` on every platform so manifests remain portable.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectPath(String);

impl ProjectPath {
    pub fn new(path: impl AsRef<str>) -> ProjectResult<Self> {
        let original = path.as_ref();
        if original.is_empty() {
            return Err(invalid_path(original, "path cannot be empty"));
        }
        if original.contains('\0') {
            return Err(invalid_path(original, "path cannot contain a NUL byte"));
        }

        let replaced = original.replace('\\', "/");
        if replaced.starts_with('/') || replaced.starts_with("//") {
            return Err(invalid_path(original, "absolute and UNC paths are not allowed"));
        }
        if has_windows_drive_prefix(&replaced) {
            return Err(invalid_path(original, "Windows drive paths are not allowed"));
        }

        let mut normalized = Vec::new();
        for component in replaced.split('/') {
            match component {
                "" | "." => {}
                ".." => return Err(invalid_path(original, "parent traversal ('..') is not allowed")),
                value => normalized.push(value),
            }
        }

        if normalized.is_empty() {
            return Err(invalid_path(original, "path must name a file or directory"));
        }

        Ok(Self(normalized.join("/")))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }

    pub fn file_name(&self) -> Option<&str> {
        self.0.rsplit('/').next()
    }

    pub fn extension(&self) -> Option<&str> {
        self.file_name()
            .and_then(|name| name.rsplit_once('.'))
            .map(|(_, extension)| extension)
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        let prefix = prefix.trim_matches('/');
        self.0 == prefix || self.0.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
    }

    pub fn join(&self, child: impl AsRef<str>) -> ProjectResult<Self> {
        Self::new(format!("{}/{}", self.0, child.as_ref()))
    }
}

impl fmt::Display for ProjectPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<Path> for ProjectPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl Borrow<str> for ProjectPath {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for ProjectPath {
    type Error = ProjectError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for ProjectPath {
    type Error = ProjectError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Serialize for ProjectPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProjectPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPaths {
    root: PathBuf,
    manifest: PathBuf,
    assets: PathBuf,
    scenes: PathBuf,
    scripts: PathBuf,
    models: PathBuf,
    textures: PathBuf,
    audio: PathBuf,
    fonts: PathBuf,
    metadata: PathBuf,
    imported: PathBuf,
    cache: PathBuf,
    editor: PathBuf,
    builds: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: impl AsRef<Path>) -> ProjectResult<Self> {
        let lexical_root = absolute_lexical(root.as_ref())?;
        let root = if lexical_root.exists() {
            fs::canonicalize(&lexical_root)
                .map_err(|error| ProjectError::io("canonicalize project root", &lexical_root, error))?
        } else {
            lexical_root
        };
        Ok(Self {
            manifest: root.join(PROJECT_MANIFEST_FILE),
            assets: root.join("assets"),
            scenes: root.join("assets/scenes"),
            scripts: root.join("assets/scripts"),
            models: root.join("assets/models"),
            textures: root.join("assets/textures"),
            audio: root.join("assets/audio"),
            fonts: root.join("assets/fonts"),
            metadata: root.join(".vetrace"),
            imported: root.join(".vetrace/imported"),
            cache: root.join(".vetrace/cache"),
            editor: root.join(".vetrace/editor"),
            builds: root.join("builds"),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &Path {
        &self.manifest
    }

    pub fn assets(&self) -> &Path {
        &self.assets
    }

    pub fn scenes(&self) -> &Path {
        &self.scenes
    }

    pub fn scripts(&self) -> &Path {
        &self.scripts
    }

    pub fn models(&self) -> &Path {
        &self.models
    }

    pub fn textures(&self) -> &Path {
        &self.textures
    }

    pub fn audio(&self) -> &Path {
        &self.audio
    }

    pub fn fonts(&self) -> &Path {
        &self.fonts
    }

    pub fn metadata(&self) -> &Path {
        &self.metadata
    }

    pub fn imported(&self) -> &Path {
        &self.imported
    }

    pub fn cache(&self) -> &Path {
        &self.cache
    }

    pub fn editor(&self) -> &Path {
        &self.editor
    }

    pub fn builds(&self) -> &Path {
        &self.builds
    }

    pub fn ensure_layout(&self) -> ProjectResult<()> {
        for path in [
            &self.root,
            &self.assets,
            &self.scenes,
            &self.scripts,
            &self.models,
            &self.textures,
            &self.audio,
            &self.fonts,
            &self.metadata,
            &self.imported,
            &self.cache,
            &self.editor,
            &self.builds,
        ] {
            fs::create_dir_all(path).map_err(|error| ProjectError::io("create directory", path, error))?;
        }
        Ok(())
    }

    /// Resolves a validated project path lexically beneath the project root.
    pub fn resolve(&self, path: &ProjectPath) -> PathBuf {
        self.root.join(path.as_path())
    }

    /// Resolves an existing path and verifies that symlinks do not escape the
    /// canonical project root.
    pub fn resolve_existing(&self, path: &ProjectPath) -> ProjectResult<PathBuf> {
        let resolved = self.resolve(path);
        let canonical_root = fs::canonicalize(&self.root)
            .map_err(|error| ProjectError::io("canonicalize project root", &self.root, error))?;
        let canonical_path = fs::canonicalize(&resolved)
            .map_err(|error| ProjectError::io("canonicalize project path", &resolved, error))?;
        if !canonical_path.starts_with(&canonical_root) {
            return Err(ProjectError::PathOutsideProject {
                root: canonical_root,
                path: canonical_path,
            });
        }
        Ok(canonical_path)
    }

    /// Resolves a path intended for writing and verifies the nearest existing
    /// parent directory. This prevents an existing parent symlink from routing
    /// output outside the project.
    pub fn resolve_for_write(&self, path: &ProjectPath) -> ProjectResult<PathBuf> {
        let resolved = self.resolve(path);
        let canonical_root = fs::canonicalize(&self.root)
            .map_err(|error| ProjectError::io("canonicalize project root", &self.root, error))?;
        let existing_parent = nearest_existing_ancestor(&resolved).ok_or_else(|| {
            ProjectError::PathOutsideProject {
                root: canonical_root.clone(),
                path: resolved.clone(),
            }
        })?;
        let canonical_parent = fs::canonicalize(existing_parent)
            .map_err(|error| ProjectError::io("canonicalize project path parent", existing_parent, error))?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(ProjectError::PathOutsideProject {
                root: canonical_root,
                path: canonical_parent,
            });
        }
        Ok(resolved)
    }

    /// Converts an absolute or root-relative filesystem path into a portable
    /// `ProjectPath`. Existing symlinks are checked when possible.
    pub fn to_project_path(&self, path: impl AsRef<Path>) -> ProjectResult<ProjectPath> {
        let path = path.as_ref();
        let lexical = if path.is_absolute() {
            absolute_lexical(path)?
        } else {
            absolute_lexical(&self.root.join(path))?
        };

        let (checked_root, checked_path) = if self.root.exists() {
            let canonical_root = fs::canonicalize(&self.root)
                .map_err(|error| ProjectError::io("canonicalize project root", &self.root, error))?;
            if lexical.exists() {
                let canonical_path = fs::canonicalize(&lexical)
                    .map_err(|error| ProjectError::io("canonicalize project path", &lexical, error))?;
                (canonical_root, canonical_path)
            } else if let Some(existing_parent) = nearest_existing_ancestor(&lexical) {
                let canonical_parent = fs::canonicalize(existing_parent)
                    .map_err(|error| ProjectError::io("canonicalize project path parent", existing_parent, error))?;
                if !canonical_parent.starts_with(&canonical_root) {
                    return Err(ProjectError::PathOutsideProject {
                        root: canonical_root,
                        path: canonical_parent,
                    });
                }
                (canonical_root, lexical)
            } else {
                (canonical_root, lexical)
            }
        } else {
            (self.root.clone(), lexical)
        };

        if !checked_path.starts_with(&checked_root) {
            return Err(ProjectError::PathOutsideProject {
                root: checked_root,
                path: checked_path,
            });
        }
        let relative = checked_path.strip_prefix(&checked_root).expect("prefix checked");
        let mut portable_parts = Vec::new();
        for component in relative.components() {
            if let Component::Normal(value) = component {
                let value = value.to_str().ok_or_else(|| ProjectError::InvalidProjectPath {
                    path: relative.display().to_string(),
                    reason: "project paths must be valid UTF-8".to_owned(),
                })?;
                portable_parts.push(value);
            }
        }
        ProjectPath::new(portable_parts.join("/"))
    }
}

fn invalid_path(path: &str, reason: impl Into<String>) -> ProjectError {
    ProjectError::InvalidProjectPath {
        path: path.to_owned(),
        reason: reason.into(),
    }
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn nearest_existing_ancestor(path: &Path) -> Option<&Path> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return Some(candidate);
        }
        current = candidate.parent();
    }
    None
}

fn absolute_lexical(path: &Path) -> ProjectResult<PathBuf> {
    let input = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let cwd = env::current_dir().map_err(|error| ProjectError::io("read current directory", ".", error))?;
        cwd.join(path)
    };

    let mut anchor = PathBuf::new();
    let mut normal_components = Vec::new();
    for component in input.components() {
        match component {
            Component::Prefix(prefix) => anchor.push(prefix.as_os_str()),
            Component::RootDir => anchor.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normal_components.pop();
            }
            Component::Normal(value) => normal_components.push(value.to_os_string()),
        }
    }
    anchor.extend(normal_components);
    Ok(anchor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_portable_paths() {
        assert_eq!(ProjectPath::new("assets\\models//./player.glb").unwrap().as_str(), "assets/models/player.glb");
    }

    #[test]
    fn rejects_unsafe_paths() {
        for path in ["", "../secret", "assets/../../secret", "/tmp/file", "C:\\file", "//server/share"] {
            assert!(ProjectPath::new(path).is_err(), "{path} should be rejected");
        }
    }
}
