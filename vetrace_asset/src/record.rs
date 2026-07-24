use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vetrace_project::ProjectPath;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(pub Uuid);

impl AssetId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
    pub fn nil() -> Self { Self(Uuid::nil()) }
    pub fn as_uuid(self) -> Uuid { self.0 }
}

impl Default for AssetId {
    fn default() -> Self { Self::nil() }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

impl FromStr for AssetId {
    type Err = uuid::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> { Uuid::parse_str(value).map(Self) }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "custom")]
pub enum AssetKind {
    Scene,
    Script,
    Model,
    Texture,
    Audio,
    Font,
    Shader,
    Material,
    Data,
    Custom(String),
    Unknown,
}

impl AssetKind {
    pub fn label(&self) -> &str {
        match self {
            Self::Scene => "Scene",
            Self::Script => "Script",
            Self::Model => "Model",
            Self::Texture => "Texture",
            Self::Audio => "Audio",
            Self::Font => "Font",
            Self::Shader => "Shader",
            Self::Material => "Material",
            Self::Data => "Data",
            Self::Custom(value) => value,
            Self::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    Ready,
    NeedsImport,
    Missing,
    Failed,
    Unsupported,
}

impl Default for AssetStatus {
    fn default() -> Self { Self::NeedsImport }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDependency {
    pub path: ProjectPath,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<AssetId>,
    #[serde(default)]
    pub missing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetDiagnostic {
    pub severity: AssetDiagnosticSeverity,
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<AssetId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ProjectPath>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency: Option<ProjectPath>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetRecord {
    pub id: AssetId,
    pub source: ProjectPath,
    pub kind: AssetKind,
    pub status: AssetStatus,
    pub source_hash: String,
    pub source_size: u64,
    pub modified_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub importer: Option<crate::ImporterStamp>,
    #[serde(default)]
    pub outputs: Vec<ProjectPath>,
    #[serde(default)]
    pub dependencies: Vec<AssetDependency>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl AssetRecord {
    pub fn file_name(&self) -> &str { self.source.file_name().unwrap_or(self.source.as_str()) }
}
