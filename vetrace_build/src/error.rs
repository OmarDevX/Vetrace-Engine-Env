use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(feature = "export")]
use vetrace_asset::AssetError;
use vetrace_project::ProjectError;

pub type BuildResult<T> = Result<T, BuildError>;

#[derive(Debug)]
pub enum BuildError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Project(ProjectError),
    #[cfg(feature = "export")]
    Asset(AssetError),
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    ConfigSerialize(toml::ser::Error),
    Json(serde_json::Error),
    Zip(zip::result::ZipError),
    Validation(String),
    InvalidPackage(String),
    MissingPlayerTemplate(PathBuf),
    UnsafeOutput(PathBuf),
}

impl BuildError {
    pub(crate) fn io(
        operation: &'static str,
        path: impl AsRef<Path>,
        source: io::Error,
    ) -> Self {
        Self::Io {
            operation,
            path: path.as_ref().to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, path, source } => {
                write!(formatter, "failed to {operation} '{}': {source}", path.display())
            }
            Self::Project(error) => error.fmt(formatter),
            #[cfg(feature = "export")]
            Self::Asset(error) => error.fmt(formatter),
            Self::ConfigParse { path, source } => write!(
                formatter,
                "failed to parse export configuration '{}': {source}",
                path.display()
            ),
            Self::ConfigSerialize(source) => {
                write!(formatter, "failed to serialize export configuration: {source}")
            }
            Self::Json(source) => write!(formatter, "failed to process build metadata: {source}"),
            Self::Zip(source) => write!(formatter, "failed to process Vetrace package: {source}"),
            Self::Validation(message) => formatter.write_str(message),
            Self::InvalidPackage(message) => write!(formatter, "invalid Vetrace package: {message}"),
            Self::MissingPlayerTemplate(path) => write!(
                formatter,
                "prebuilt vetrace-player template was not found at '{}'",
                path.display()
            ),
            Self::UnsafeOutput(path) => write!(
                formatter,
                "export output '{}' is outside the project or otherwise unsafe",
                path.display()
            ),
        }
    }
}

impl Error for BuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Project(source) => Some(source),
            #[cfg(feature = "export")]
            Self::Asset(source) => Some(source),
            Self::ConfigParse { source, .. } => Some(source),
            Self::ConfigSerialize(source) => Some(source),
            Self::Json(source) => Some(source),
            Self::Zip(source) => Some(source),
            _ => None,
        }
    }
}

impl From<ProjectError> for BuildError {
    fn from(value: ProjectError) -> Self { Self::Project(value) }
}

#[cfg(feature = "export")]
impl From<AssetError> for BuildError {
    fn from(value: AssetError) -> Self { Self::Asset(value) }
}

impl From<serde_json::Error> for BuildError {
    fn from(value: serde_json::Error) -> Self { Self::Json(value) }
}

impl From<zip::result::ZipError> for BuildError {
    fn from(value: zip::result::ZipError) -> Self { Self::Zip(value) }
}
