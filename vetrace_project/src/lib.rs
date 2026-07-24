//! Project-level configuration for Vetrace applications.
//!
//! This crate deliberately has no dependency on ECS, rendering, windowing,
//! physics, scripting, or editor crates. It provides the stable product
//! boundary used by Vetrace Studio, the runtime, build tools, and player.

mod discovery;
mod error;
mod features;
mod input;
mod manifest;
mod migration;
mod paths;
mod project;
mod settings;
mod validation;

pub use discovery::{discover_projects, find_project_root};
pub use error::ProjectError;
pub use features::FeatureSettings;
pub use input::{AxisDirection, InputAction, InputAxisBinding, InputMap};
pub use migration::{migrate_project, ProjectMigrationReport};
pub use manifest::{
    ProjectInfo, ProjectManifest, CURRENT_PROJECT_FORMAT_VERSION, PROJECT_MANIFEST_FILE,
};
pub use paths::{ProjectPath, ProjectPaths};
pub use project::VetraceProject;
pub use settings::{
    AdapterPreference, AmbientOcclusion, AntiAliasing, ApplicationSettings, GiMode,
    PhysicsSettings, PresentMode, RenderingBackend, RenderingSettings, RuntimeSettings,
    ScriptLanguage, ScriptingSettings, ShadowQuality,
};
pub use validation::{
    validate_manifest, validate_project_files, ValidationIssue, ValidationReport, ValidationSeverity,
};

pub type ProjectResult<T> = Result<T, ProjectError>;
