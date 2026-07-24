use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ApplicationSettings, FeatureSettings, InputMap, PhysicsSettings, RenderingSettings, RuntimeSettings,
    ScriptingSettings,
};

pub const CURRENT_PROJECT_FORMAT_VERSION: u32 = 1;
pub const PROJECT_MANIFEST_FILE: &str = "project.vetrace.toml";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectManifest {
    pub format_version: u32,
    pub project: ProjectInfo,
    pub application: ApplicationSettings,
    pub runtime: RuntimeSettings,
    pub features: FeatureSettings,
    pub scripting: ScriptingSettings,
    pub rendering: RenderingSettings,
    pub physics: PhysicsSettings,
    pub input: InputMap,
}

impl ProjectManifest {
    pub fn new(name: impl Into<String>, engine_version: impl Into<String>) -> Self {
        let name = name.into();
        let mut manifest = Self::default();
        manifest.project.name = name.clone();
        manifest.project.engine_version = engine_version.into();
        manifest.application.title = name;
        manifest
    }
}

impl Default for ProjectManifest {
    fn default() -> Self {
        Self {
            format_version: CURRENT_PROJECT_FORMAT_VERSION,
            project: ProjectInfo {
                id: Uuid::new_v4(),
                ..ProjectInfo::default()
            },
            application: ApplicationSettings::default(),
            runtime: RuntimeSettings::default(),
            features: FeatureSettings::default(),
            scripting: ScriptingSettings::default(),
            rendering: RenderingSettings::default(),
            physics: PhysicsSettings::default(),
            input: InputMap::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectInfo {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub engine_version: String,
}

impl Default for ProjectInfo {
    fn default() -> Self {
        Self {
            // A missing ID in a parsed manifest must not silently generate a
            // different identity on every load. Validation reports the nil ID.
            id: Uuid::nil(),
            name: "New Vetrace Project".to_owned(),
            version: "0.1.0".to_owned(),
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}
