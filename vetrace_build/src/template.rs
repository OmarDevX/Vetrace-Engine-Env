use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use vetrace_project::VetraceProject;

use crate::{BuildError, BuildResult, ExportTarget, VPAK_FORMAT_VERSION};

pub const PLAYER_TEMPLATE_METADATA_FORMAT_VERSION: u32 = 1;
pub const PLAYER_TEMPLATE_METADATA_SUFFIX: &str = ".template.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlayerTemplateTarget {
    LinuxX86_64,
    WindowsX86_64,
}

impl PlayerTemplateTarget {
    pub fn current() -> Option<Self> {
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(Self::LinuxX86_64)
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some(Self::WindowsX86_64)
        } else {
            None
        }
    }

    pub fn for_export_target(target: ExportTarget) -> Option<Self> {
        match target {
            ExportTarget::Host => Self::current(),
            ExportTarget::LinuxX86_64 => Some(Self::LinuxX86_64),
            ExportTarget::WindowsX86_64 => Some(Self::WindowsX86_64),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTemplateMetadata {
    pub format_version: u32,
    pub engine_version: String,
    pub target: PlayerTemplateTarget,
    pub vpak_format_version: u32,
    pub features: BTreeSet<String>,
}

impl PlayerTemplateMetadata {
    pub fn compiled_for_current_player() -> BuildResult<Self> {
        let target = PlayerTemplateTarget::current().ok_or_else(|| {
            BuildError::Validation("this host target is not supported by player-template metadata".to_owned())
        })?;
        let mut features = BTreeSet::new();
        // This helper describes the standard full player template used by the
        // build tests and release tooling. Custom/minimal templates should
        // construct metadata explicitly from their compiled feature set.
        for feature in [
            "rendering", "physics", "audio", "animation", "networking", "ui", "scripting",
        ] {
            features.insert(feature.to_owned());
        }
        Ok(Self {
            format_version: PLAYER_TEMPLATE_METADATA_FORMAT_VERSION,
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            target,
            vpak_format_version: VPAK_FORMAT_VERSION,
            features,
        })
    }

    pub fn validate_for_project(
        &self,
        project: &VetraceProject,
        target: ExportTarget,
    ) -> BuildResult<()> {
        if self.format_version != PLAYER_TEMPLATE_METADATA_FORMAT_VERSION {
            return Err(BuildError::Validation(format!(
                "unsupported player-template metadata format {}; expected {}",
                self.format_version, PLAYER_TEMPLATE_METADATA_FORMAT_VERSION
            )));
        }
        if self.vpak_format_version != VPAK_FORMAT_VERSION {
            return Err(BuildError::Validation(format!(
                "player template supports VPAK format {}, but this exporter writes format {}",
                self.vpak_format_version, VPAK_FORMAT_VERSION
            )));
        }
        let expected = PlayerTemplateTarget::for_export_target(target).ok_or_else(|| {
            BuildError::Validation(format!("export target '{}' is unsupported by player templates", target.label()))
        })?;
        if self.target != expected {
            return Err(BuildError::Validation(format!(
                "player template target {:?} is incompatible with export target {:?}",
                self.target, expected
            )));
        }
        let project_engine = project.manifest().project.engine_version.trim();
        if project_engine != self.engine_version {
            return Err(BuildError::Validation(format!(
                "player template engine version '{}' does not match project engine version '{}'",
                self.engine_version, project_engine
            )));
        }
        let required = required_project_features(project);
        let missing = required.difference(&self.features).cloned().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(BuildError::Validation(format!(
                "player template is missing required feature(s): {}",
                missing.join(", ")
            )));
        }
        Ok(())
    }
}

pub fn player_template_metadata_path(player_template: &Path) -> PathBuf {
    let file_name = player_template
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "vetrace-player".to_owned());
    player_template.with_file_name(format!("{file_name}{PLAYER_TEMPLATE_METADATA_SUFFIX}"))
}

pub fn load_player_template_metadata(player_template: &Path) -> BuildResult<PlayerTemplateMetadata> {
    let path = player_template_metadata_path(player_template);
    let bytes = fs::read(&path)
        .map_err(|error| BuildError::io("read player-template metadata", &path, error))?;
    let metadata = serde_json::from_slice(&bytes)?;
    Ok(metadata)
}

pub fn write_player_template_metadata(
    player_template: &Path,
    metadata: &PlayerTemplateMetadata,
) -> BuildResult<PathBuf> {
    let path = player_template_metadata_path(player_template);
    let bytes = serde_json::to_vec_pretty(metadata)?;
    fs::write(&path, bytes)
        .map_err(|error| BuildError::io("write player-template metadata", &path, error))?;
    Ok(path)
}

pub fn validate_player_template(
    player_template: &Path,
    project: &VetraceProject,
    target: ExportTarget,
) -> BuildResult<PlayerTemplateMetadata> {
    if !player_template.is_file() {
        return Err(BuildError::MissingPlayerTemplate(player_template.to_path_buf()));
    }
    let metadata = load_player_template_metadata(player_template).map_err(|error| {
        BuildError::Validation(format!(
            "player template '{}' requires sidecar metadata '{}': {error}",
            player_template.display(),
            player_template_metadata_path(player_template).display()
        ))
    })?;
    metadata.validate_for_project(project, target)?;
    Ok(metadata)
}

fn required_project_features(project: &VetraceProject) -> BTreeSet<String> {
    let features = &project.manifest().features;
    let mut required = BTreeSet::new();
    for (name, enabled) in [
        ("rendering", features.rendering),
        ("physics", features.physics),
        ("audio", features.audio),
        ("animation", features.animation),
        ("networking", features.networking),
        ("ui", features.ui),
        ("scripting", features.scripting),
    ] {
        if enabled { required.insert(name.to_owned()); }
    }
    required
}

pub fn sanitize_executable_name(value: &str) -> String {
    let mut output = String::new();
    let mut last_separator = false;
    for character in value.chars() {
        let valid = character.is_ascii_alphanumeric() || matches!(character, '-' | '_');
        if valid {
            output.push(character);
            last_separator = false;
        } else if !last_separator && !output.is_empty() {
            output.push('-');
            last_separator = true;
        }
    }
    while output.ends_with('-') { output.pop(); }
    if output.is_empty() { "VetraceGame".to_owned() } else { output }
}

pub fn default_executable_name(project_name: &str, target: ExportTarget) -> String {
    let mut name = sanitize_executable_name(project_name);
    if target.resolves_to_windows() && !name.to_ascii_lowercase().ends_with(".exe") {
        name.push_str(".exe");
    }
    name
}

pub fn find_player_template() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VETRACE_PLAYER_TEMPLATE") {
        let path = PathBuf::from(path);
        if path.is_file() { return Some(path); }
    }
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let names: &[&str] = if cfg!(windows) {
        &["vetrace-player.exe", "vetrace_player.exe"]
    } else {
        &["vetrace-player", "vetrace_player"]
    };
    names.iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executable_names_are_safe() {
        assert_eq!(sanitize_executable_name("My Cool: Game"), "My-Cool-Game");
        assert_eq!(sanitize_executable_name("../../"), "VetraceGame");
    }

    #[test]
    fn metadata_sidecar_is_next_to_binary() {
        assert_eq!(
            player_template_metadata_path(Path::new("bin/vetrace-player")),
            PathBuf::from("bin/vetrace-player.template.json")
        );
    }
}
