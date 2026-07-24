use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::ValidationReport;

#[derive(Debug)]
pub enum ProjectError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    ManifestNotFound {
        start: PathBuf,
        file_name: &'static str,
    },
    ManifestAlreadyExists(PathBuf),
    ParseManifest {
        path: PathBuf,
        source: toml::de::Error,
    },
    SerializeManifest(toml::ser::Error),
    InvalidProjectPath {
        path: String,
        reason: String,
    },
    PathOutsideProject {
        root: PathBuf,
        path: PathBuf,
    },
    Validation(ValidationReport),
}

impl ProjectError {
    pub(crate) fn io(operation: &'static str, path: impl AsRef<Path>, source: io::Error) -> Self {
        Self::Io {
            operation,
            path: path.as_ref().to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(formatter, "failed to {operation} '{}': {source}", path.display()),
            Self::ManifestNotFound { start, file_name } => write!(
                formatter,
                "could not find {file_name} at or above '{}'",
                start.display()
            ),
            Self::ManifestAlreadyExists(path) => {
                write!(formatter, "a Vetrace project manifest already exists at '{}'", path.display())
            }
            Self::ParseManifest { path, source } => {
                write!(formatter, "failed to parse project manifest '{}': {source}", path.display())
            }
            Self::SerializeManifest(source) => {
                write!(formatter, "failed to serialize project manifest: {source}")
            }
            Self::InvalidProjectPath { path, reason } => {
                write!(formatter, "invalid project-relative path '{path}': {reason}")
            }
            Self::PathOutsideProject { root, path } => write!(
                formatter,
                "path '{}' resolves outside project root '{}'",
                path.display(),
                root.display()
            ),
            Self::Validation(report) => write!(
                formatter,
                "project validation failed with {} error(s) and {} warning(s)",
                report.error_count(),
                report.warning_count()
            ),
        }
    }
}

impl Error for ProjectError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::ParseManifest { source, .. } => Some(source),
            Self::SerializeManifest(source) => Some(source),
            _ => None,
        }
    }
}
