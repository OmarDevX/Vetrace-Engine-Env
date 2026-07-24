use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum AssetError {
    Io { operation: &'static str, path: PathBuf, source: std::io::Error },
    Database(String),
    Importer(String),
    UnknownAsset(String),
    UnsupportedAsset(String),
    Watch(String),
    InvalidPath(String),
}

impl AssetError {
    pub(crate) fn io(operation: &'static str, path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io { operation, path: path.into(), source }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, path, source } => {
                write!(f, "failed to {operation} '{}': {source}", path.display())
            }
            Self::Database(message) => write!(f, "asset database error: {message}"),
            Self::Importer(message) => write!(f, "asset importer error: {message}"),
            Self::UnknownAsset(asset) => write!(f, "unknown asset: {asset}"),
            Self::UnsupportedAsset(asset) => write!(f, "no importer is registered for {asset}"),
            Self::Watch(message) => write!(f, "asset watcher error: {message}"),
            Self::InvalidPath(message) => write!(f, "invalid asset path: {message}"),
        }
    }
}

impl Error for AssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for AssetError {
    fn from(error: serde_json::Error) -> Self { Self::Database(error.to_string()) }
}

pub type AssetResult<T> = Result<T, AssetError>;
