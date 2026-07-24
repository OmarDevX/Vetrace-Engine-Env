use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

use vetrace_build::BuildError;
use vetrace_project::ProjectError;
use vetrace_runtime::RuntimeError;

pub const EXIT_SUCCESS: u8 = 0;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_PROJECT: u8 = 3;
pub const EXIT_RUNTIME_SETUP: u8 = 4;
pub const EXIT_RUNTIME_EXECUTION: u8 = 5;

#[derive(Debug)]
pub enum PlayerError {
    WorkingDirectory {
        path: PathBuf,
        source: io::Error,
    },
    Package(BuildError),
    InvalidFixedDelta(f32),
    Project(ProjectError),
    RuntimeSetup(RuntimeError),
    RuntimeExecution(RuntimeError),
}

impl PlayerError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::WorkingDirectory { .. } | Self::InvalidFixedDelta(_) => EXIT_USAGE,
            Self::Project(_) | Self::Package(_) => EXIT_PROJECT,
            Self::RuntimeSetup(_) => EXIT_RUNTIME_SETUP,
            Self::RuntimeExecution(_) => EXIT_RUNTIME_EXECUTION,
        }
    }
}

impl fmt::Display for PlayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkingDirectory { path, source } => write!(
                formatter,
                "failed to change working directory to '{}': {source}",
                path.display()
            ),
            Self::Package(error) => write!(formatter, "failed to open package: {error}"),
            Self::InvalidFixedDelta(delta) => write!(
                formatter,
                "player timestep must be finite and greater than zero, got {delta}"
            ),
            Self::Project(error) => write!(formatter, "{error}"),
            Self::RuntimeSetup(error) => write!(formatter, "failed to create runtime: {error}"),
            Self::RuntimeExecution(error) => write!(formatter, "runtime failed: {error}"),
        }
    }
}

impl Error for PlayerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::WorkingDirectory { source, .. } => Some(source),
            Self::Package(error) => Some(error),
            Self::Project(error) => Some(error),
            Self::RuntimeSetup(error) | Self::RuntimeExecution(error) => Some(error),
            Self::InvalidFixedDelta(_) => None,
        }
    }
}

impl From<ProjectError> for PlayerError {
    fn from(value: ProjectError) -> Self { Self::Project(value) }
}

impl From<BuildError> for PlayerError {
    fn from(value: BuildError) -> Self { Self::Package(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_classes_have_stable_exit_codes() {
        assert_eq!(PlayerError::InvalidFixedDelta(0.0).exit_code(), EXIT_USAGE);
        assert_eq!(
            PlayerError::RuntimeSetup(RuntimeError::SceneNotLoaded).exit_code(),
            EXIT_RUNTIME_SETUP
        );
        assert_eq!(
            PlayerError::RuntimeExecution(RuntimeError::SceneNotLoaded).exit_code(),
            EXIT_RUNTIME_EXECUTION
        );
    }
}
