use std::error::Error;
use std::fmt;
use std::path::PathBuf;

use vetrace_project::{ProjectError, ProjectPath};

use crate::RuntimeState;

#[derive(Debug)]
pub enum RuntimeError {
    Project(ProjectError),
    InvalidState {
        operation: &'static str,
        state: RuntimeState,
    },
    InvalidDelta(f32),
    FeatureUnavailable {
        feature: &'static str,
        required_cargo_feature: &'static str,
    },
    Plugin(String),
    SceneLoad {
        path: PathBuf,
        message: String,
    },
    SceneNotLoaded,
    ScriptLoad {
        script: ProjectPath,
        message: String,
    },
    ScriptBinding {
        entity: u64,
        script: String,
        message: String,
    },
    MissingResource(&'static str),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Project(error) => write!(formatter, "{error}"),
            Self::InvalidState { operation, state } => {
                write!(formatter, "cannot {operation} while runtime is {state:?}")
            }
            Self::InvalidDelta(delta) => write!(
                formatter,
                "runtime delta time must be finite and non-negative, got {delta}"
            ),
            Self::FeatureUnavailable { feature, required_cargo_feature } => write!(
                formatter,
                "project requires {feature}, but this runtime was built without Cargo feature `{required_cargo_feature}`"
            ),
            Self::Plugin(message) => write!(formatter, "runtime plugin failure: {message}"),
            Self::SceneLoad { path, message } => {
                write!(formatter, "failed to load scene '{}': {message}", path.display())
            }
            Self::SceneNotLoaded => formatter.write_str("runtime has no active scene"),
            Self::ScriptLoad { script, message } => {
                write!(formatter, "failed to load Lua script '{script}': {message}")
            }
            Self::ScriptBinding { entity, script, message } => write!(
                formatter,
                "failed to bind Lua script '{script}' to entity {entity}: {message}"
            ),
            Self::MissingResource(resource) => {
                write!(formatter, "runtime resource `{resource}` is missing")
            }
        }
    }
}

impl Error for RuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Project(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ProjectError> for RuntimeError {
    fn from(value: ProjectError) -> Self { Self::Project(value) }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;
