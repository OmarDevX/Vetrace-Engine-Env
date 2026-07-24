use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    validate_manifest, ProjectError, ProjectManifest, ProjectPaths, ProjectResult,
    CURRENT_PROJECT_FORMAT_VERSION, PROJECT_MANIFEST_FILE,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMigrationReport {
    pub from_version: u32,
    pub to_version: u32,
    pub backup: Option<PathBuf>,
    pub changes: Vec<String>,
}

impl ProjectMigrationReport {
    pub fn changed(&self) -> bool { self.from_version != self.to_version || !self.changes.is_empty() }
}

/// Migrates a project manifest in place and writes a timestamp-free `.bak`
/// copy before replacing the authored file. Newer formats are never
/// downgraded.
pub fn migrate_project(path: impl AsRef<Path>) -> ProjectResult<ProjectMigrationReport> {
    let path = path.as_ref();
    let manifest_path = if path.file_name().and_then(|name| name.to_str()) == Some(PROJECT_MANIFEST_FILE) {
        path.to_path_buf()
    } else {
        path.join(PROJECT_MANIFEST_FILE)
    };
    let source = fs::read_to_string(&manifest_path)
        .map_err(|error| ProjectError::io("read project manifest for migration", &manifest_path, error))?;
    let mut value: toml::Value = toml::from_str(&source).map_err(|source| ProjectError::ParseManifest {
        path: manifest_path.clone(),
        source,
    })?;
    let from_version = value
        .get("format_version")
        .and_then(toml::Value::as_integer)
        .unwrap_or(0)
        .max(0) as u32;
    if from_version > CURRENT_PROJECT_FORMAT_VERSION {
        return Err(ProjectError::InvalidProjectPath {
            path: manifest_path.display().to_string(),
            reason: format!(
                "project format {from_version} is newer than supported format {}",
                CURRENT_PROJECT_FORMAT_VERSION
            ),
        });
    }

    let mut report = ProjectMigrationReport {
        from_version,
        to_version: CURRENT_PROJECT_FORMAT_VERSION,
        ..ProjectMigrationReport::default()
    };
    if from_version == CURRENT_PROJECT_FORMAT_VERSION {
        // Parsing through the strongly typed manifest still checks that the
        // current document shape is valid.
        let manifest: ProjectManifest = toml::from_str(&source).map_err(|source| ProjectError::ParseManifest {
            path: manifest_path,
            source,
        })?;
        validate_manifest(&manifest).into_result()?;
        return Ok(report);
    }

    migrate_value(&mut value, from_version, &mut report);
    let migrated_source = toml::to_string_pretty(&value).map_err(ProjectError::SerializeManifest)?;
    let manifest: ProjectManifest = toml::from_str(&migrated_source).map_err(|source| ProjectError::ParseManifest {
        path: manifest_path.clone(),
        source,
    })?;

    validate_manifest(&manifest).into_result()?;
    let root = manifest_path.parent().unwrap_or(Path::new("."));
    ProjectPaths::new(root)?.ensure_layout()?;
    let backup = manifest_path.with_extension("toml.bak");
    fs::copy(&manifest_path, &backup)
        .map_err(|error| ProjectError::io("backup project manifest", &backup, error))?;
    report.backup = Some(backup);
    write_atomic(&manifest_path, migrated_source.as_bytes())?;
    Ok(report)
}

fn migrate_value(value: &mut toml::Value, from_version: u32, report: &mut ProjectMigrationReport) {
    let Some(table) = value.as_table_mut() else { return; };
    if from_version == 0 {
        // Early prototypes stored subsystem switches under `[runtime]`.
        // Preserve any such values while moving them to `[features]`.
        let mut feature_values = toml::map::Map::new();
        if let Some(runtime) = table.get_mut("runtime").and_then(toml::Value::as_table_mut) {
            for name in ["rendering", "physics", "audio", "animation", "networking", "ui", "scripting"] {
                if let Some(value) = runtime.remove(name) {
                    feature_values.insert(name.to_owned(), value);
                    report.changes.push(format!("moved runtime.{name} to features.{name}"));
                }
            }
        }
        let features = table
            .entry("features".to_owned())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        if let Some(features) = features.as_table_mut() {
            for (name, value) in feature_values { features.entry(name).or_insert(value); }
        }
        report.changes.push("upgraded prototype project manifest to format 1".to_owned());
    }
    table.insert(
        "format_version".to_owned(),
        toml::Value::Integer(CURRENT_PROJECT_FORMAT_VERSION as i64),
    );
}


fn write_atomic(path: &Path, contents: &[u8]) -> ProjectResult<()> {
    use std::io::Write;

    let temporary = path.with_file_name(format!(".{PROJECT_MANIFEST_FILE}.migration.tmp"));
    let mut file = fs::File::create(&temporary)
        .map_err(|error| ProjectError::io("create migrated project manifest", &temporary, error))?;
    file.write_all(contents)
        .map_err(|error| ProjectError::io("write migrated project manifest", &temporary, error))?;
    file.sync_all()
        .map_err(|error| ProjectError::io("flush migrated project manifest", &temporary, error))?;
    drop(file);
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| ProjectError::io("replace migrated project manifest", path, error))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| ProjectError::io("replace migrated project manifest", path, error))
}
