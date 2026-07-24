use std::fs;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use vetrace_project::{ProjectPath, VetraceProject};

use crate::{BuildError, BuildResult};

pub const EXPORT_CONFIG_FORMAT_VERSION: u32 = 1;
pub const EXPORT_CONFIG_FILE: &str = "export.vetrace.toml";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportTarget {
    #[default]
    Host,
    LinuxX86_64,
    WindowsX86_64,
}

impl ExportTarget {
    pub const ALL: [Self; 3] = [Self::Host, Self::LinuxX86_64, Self::WindowsX86_64];

    pub fn label(self) -> &'static str {
        match self {
            Self::Host => "Host platform",
            Self::LinuxX86_64 => "Linux x86_64",
            Self::WindowsX86_64 => "Windows x86_64",
        }
    }

    pub fn resolves_to_windows(self) -> bool {
        matches!(self, Self::WindowsX86_64) || (matches!(self, Self::Host) && cfg!(windows))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionMode {
    Stored,
    #[default]
    Deflate,
}

impl CompressionMode {
    pub const ALL: [Self; 2] = [Self::Stored, Self::Deflate];

    pub fn label(self) -> &'static str {
        match self {
            Self::Stored => "Stored (fastest)",
            Self::Deflate => "Deflate (smaller)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportPreset {
    pub name: String,
    pub target: ExportTarget,
    pub output_directory: ProjectPath,
    pub executable_name: String,
    pub package_name: String,
    pub compression: CompressionMode,
    pub include_asset_database: bool,
}

impl Default for ExportPreset {
    fn default() -> Self {
        Self {
            name: "Desktop".to_owned(),
            target: ExportTarget::Host,
            output_directory: ProjectPath::new("builds/desktop")
                .expect("default export directory is valid"),
            executable_name: String::new(),
            package_name: "game.vpak".to_owned(),
            compression: CompressionMode::Deflate,
            include_asset_database: true,
        }
    }
}

impl ExportPreset {
    pub fn validate(&self) -> BuildResult<()> {
        if self.name.trim().is_empty() {
            return Err(BuildError::Validation("export preset name cannot be empty".to_owned()));
        }
        if !self.output_directory.starts_with("builds")
            || self.output_directory.as_str() == "builds"
        {
            return Err(BuildError::Validation(
                "export output directory must be a subdirectory beneath builds/".to_owned(),
            ));
        }
        if self.package_name.trim().is_empty()
            || !self.package_name.to_ascii_lowercase().ends_with(".vpak")
            || self.package_name.contains('/')
            || self.package_name.contains('\\')
        {
            return Err(BuildError::Validation(
                "package name must be a plain file name ending in .vpak".to_owned(),
            ));
        }
        if self.executable_name.contains('/') || self.executable_name.contains('\\') {
            return Err(BuildError::Validation(
                "executable name cannot contain path separators".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    pub format_version: u32,
    pub default_preset: String,
    pub presets: Vec<ExportPreset>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format_version: EXPORT_CONFIG_FORMAT_VERSION,
            default_preset: "Desktop".to_owned(),
            presets: vec![ExportPreset::default()],
        }
    }
}

impl ExportConfig {
    pub fn path(project: &VetraceProject) -> std::path::PathBuf {
        project.root().join(EXPORT_CONFIG_FILE)
    }

    pub fn load_or_default(project: &VetraceProject) -> BuildResult<Self> {
        let path = Self::path(project);
        if !path.is_file() {
            return Ok(Self::default());
        }
        let source = fs::read_to_string(&path)
            .map_err(|error| BuildError::io("read export configuration", &path, error))?;
        let config: Self = toml::from_str(&source)
            .map_err(|source| BuildError::ConfigParse { path, source })?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, project: &VetraceProject) -> BuildResult<()> {
        self.validate()?;
        let path = Self::path(project);
        let source = toml::to_string_pretty(self).map_err(BuildError::ConfigSerialize)?;
        write_atomic(&path, source.as_bytes())
    }

    pub fn validate(&self) -> BuildResult<()> {
        if self.format_version != EXPORT_CONFIG_FORMAT_VERSION {
            return Err(BuildError::Validation(format!(
                "unsupported export configuration format {}; expected {}",
                self.format_version, EXPORT_CONFIG_FORMAT_VERSION
            )));
        }
        if self.presets.is_empty() {
            return Err(BuildError::Validation(
                "export configuration must contain at least one preset".to_owned(),
            ));
        }
        let mut names = std::collections::BTreeSet::new();
        for preset in &self.presets {
            preset.validate()?;
            if !names.insert(preset.name.trim().to_ascii_lowercase()) {
                return Err(BuildError::Validation(format!(
                    "duplicate export preset name '{}'",
                    preset.name
                )));
            }
        }
        if !self.presets.iter().any(|preset| preset.name == self.default_preset) {
            return Err(BuildError::Validation(format!(
                "default export preset '{}' does not exist",
                self.default_preset
            )));
        }
        Ok(())
    }

    pub fn preset(&self, name: &str) -> Option<&ExportPreset> {
        self.presets.iter().find(|preset| preset.name == name)
    }

    pub fn upsert(&mut self, preset: ExportPreset) -> BuildResult<()> {
        preset.validate()?;
        if let Some(existing) = self.presets.iter_mut().find(|value| value.name == preset.name) {
            *existing = preset;
        } else {
            self.presets.push(preset);
        }
        self.validate()
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> BuildResult<()> {
    let temporary = path.with_extension("toml.tmp");
    {
        let mut file = fs::File::create(&temporary)
            .map_err(|error| BuildError::io("create temporary export configuration", &temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| BuildError::io("write temporary export configuration", &temporary, error))?;
        file.sync_all()
            .map_err(|error| BuildError::io("sync temporary export configuration", &temporary, error))?;
    }
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| BuildError::io("replace export configuration", path, error))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| BuildError::io("replace export configuration", path, error))?;
    Ok(())
}
