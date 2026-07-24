use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use vetrace_asset::{AssetDiagnosticSeverity, AssetManager};
use vetrace_project::VetraceProject;

use crate::{
    create_vpak, default_executable_name, sanitize_executable_name, validate_player_template,
    BuildError, BuildResult, ExportPreset, PackageOptions,
};

pub const BUILD_REPORT_FILE: &str = "build-report.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BuildAssetPreflight {
    #[default]
    Refresh,
    ExistingDatabase,
}

#[derive(Clone, Debug)]
pub struct BuildRequest {
    pub project: VetraceProject,
    pub preset: ExportPreset,
    pub player_template: PathBuf,
    pub asset_preflight: BuildAssetPreflight,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildReport {
    pub project_name: String,
    pub project_version: String,
    pub preset_name: String,
    pub output_directory: PathBuf,
    pub executable: PathBuf,
    pub package: PathBuf,
    pub package_entries: usize,
    pub package_bytes: u64,
    pub package_blake3: String,
    pub built_unix_ms: u64,
    pub warnings: Vec<String>,
}

pub fn build_project(request: &BuildRequest) -> BuildResult<BuildReport> {
    request.preset.validate()?;
    validate_player_template(
        &request.player_template,
        &request.project,
        request.preset.target,
    )?;
    let validation = request.project.validate_files();
    if !validation.is_valid() {
        return Err(BuildError::Project(vetrace_project::ProjectError::Validation(validation)));
    }
    let mut assets = AssetManager::open(&request.project)?;
    if request.asset_preflight == BuildAssetPreflight::Refresh {
        assets.refresh()?;
    }
    let blocking = assets.database().diagnostics.iter()
        .filter(|diagnostic| diagnostic.severity == AssetDiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.message.clone())
        .collect::<Vec<_>>();
    if !blocking.is_empty() {
        return Err(BuildError::Validation(format!(
            "asset preflight failed:\n- {}",
            blocking.join("\n- ")
        )));
    }

    let output = request.project.paths()
        .resolve_for_write(&request.preset.output_directory)
        .map_err(|_| BuildError::UnsafeOutput(
            request.project.root().join(request.preset.output_directory.as_path())
        ))?;
    let builds = fs::canonicalize(request.project.paths().builds())
        .unwrap_or_else(|_| request.project.paths().builds().to_path_buf());
    let lexical_output = request.project.root().join(request.preset.output_directory.as_path());
    if !lexical_output.starts_with(request.project.paths().builds()) && !output.starts_with(&builds) {
        return Err(BuildError::UnsafeOutput(output));
    }

    let parent = output.parent().unwrap_or(request.project.paths().builds());
    fs::create_dir_all(parent)
        .map_err(|error| BuildError::io("create export parent directory", parent, error))?;
    let staging = parent.join(format!(
        ".vetrace-export-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&staging)
        .map_err(|error| BuildError::io("create export staging directory", &staging, error))?;

    let mut report = match build_into_staging(request, &staging) {
        Ok(report) => report,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };

    let executable_name = report.executable.file_name()
        .ok_or_else(|| BuildError::Validation("exported executable has no file name".to_owned()))?
        .to_owned();
    let package_name = report.package.file_name()
        .ok_or_else(|| BuildError::Validation("exported package has no file name".to_owned()))?
        .to_owned();
    report.output_directory = output.clone();
    report.executable = output.join(executable_name);
    report.package = output.join(package_name);
    if let Err(error) = write_report(&staging.join(BUILD_REPORT_FILE), &report) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    publish_staging_directory(&staging, &output)?;
    Ok(report)
}


fn publish_staging_directory(staging: &Path, output: &Path) -> BuildResult<()> {
    let backup = output.with_file_name(format!(
        ".vetrace-export-backup-{}",
        uuid::Uuid::new_v4()
    ));
    let had_previous = output.exists();
    if had_previous {
        fs::rename(output, &backup)
            .map_err(|error| BuildError::io("move previous export aside", output, error))?;
    }
    if let Err(error) = fs::rename(staging, output) {
        if had_previous {
            let _ = fs::rename(&backup, output);
        }
        let _ = fs::remove_dir_all(staging);
        return Err(BuildError::io("publish export directory", output, error));
    }
    if had_previous {
        let cleanup = if backup.is_dir() {
            fs::remove_dir_all(&backup)
        } else {
            fs::remove_file(&backup)
        };
        // The new export has already been published. A stale hidden backup is
        // preferable to reporting a false build failure after success.
        let _ = cleanup;
    }
    Ok(())
}

fn build_into_staging(request: &BuildRequest, staging: &Path) -> BuildResult<BuildReport> {
    let configured_name = request.preset.executable_name.trim();
    let executable_name = if configured_name.is_empty() {
        default_executable_name(
            &request.project.manifest().project.name,
            request.preset.target,
        )
    } else {
        let mut name = sanitize_executable_name(configured_name.trim_end_matches(".exe"));
        if request.preset.target.resolves_to_windows() {
            name.push_str(".exe");
        }
        name
    };
    let executable = staging.join(&executable_name);
    fs::copy(&request.player_template, &executable)
        .map_err(|error| BuildError::io("copy player template", &executable, error))?;
    copy_permissions(&request.player_template, &executable)?;

    let package = staging.join(&request.preset.package_name);
    let package_manifest = create_vpak(
        &request.project,
        &package,
        PackageOptions {
            compression: request.preset.compression,
            include_asset_database: request.preset.include_asset_database,
        },
    )?;
    let package_bytes = fs::metadata(&package)
        .map_err(|error| BuildError::io("inspect generated package", &package, error))?
        .len();
    let package_hash = hash_file(&package)?;

    let mut warnings = Vec::new();
    let licenses = request.player_template.parent().map(|parent| parent.join("licenses"));
    if let Some(licenses) = licenses.filter(|path| path.is_dir()) {
        copy_directory(&licenses, &staging.join("licenses"))?;
    } else {
        warnings.push("No licenses directory was found beside the player template.".to_owned());
    }

    Ok(BuildReport {
        project_name: request.project.manifest().project.name.clone(),
        project_version: request.project.manifest().project.version.clone(),
        preset_name: request.preset.name.clone(),
        output_directory: staging.to_path_buf(),
        executable,
        package,
        package_entries: package_manifest.entries.len(),
        package_bytes,
        package_blake3: package_hash,
        built_unix_ms: now_unix_ms(),
        warnings,
    })
}

fn copy_permissions(source: &Path, destination: &Path) -> BuildResult<()> {
    let permissions = fs::metadata(source)
        .map_err(|error| BuildError::io("read player template permissions", source, error))?
        .permissions();
    fs::set_permissions(destination, permissions)
        .map_err(|error| BuildError::io("set exported player permissions", destination, error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(destination)
            .map_err(|error| BuildError::io("read exported player permissions", destination, error))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(destination, permissions)
            .map_err(|error| BuildError::io("make exported player executable", destination, error))?;
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> BuildResult<()> {
    fs::create_dir_all(destination)
        .map_err(|error| BuildError::io("create export directory", destination, error))?;
    for entry in fs::read_dir(source)
        .map_err(|error| BuildError::io("read export source directory", source, error))?
    {
        let entry = entry.map_err(|error| BuildError::io("read export source entry", source, error))?;
        let file_type = entry.file_type()
            .map_err(|error| BuildError::io("inspect export source entry", entry.path(), error))?;
        if file_type.is_symlink() { continue; }
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target)
                .map_err(|error| BuildError::io("copy export support file", &target, error))?;
        }
    }
    Ok(())
}

fn write_report(path: &Path, report: &BuildReport) -> BuildResult<()> {
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(report)?;
    {
        let mut file = fs::File::create(&temporary)
            .map_err(|error| BuildError::io("create temporary build report", &temporary, error))?;
        file.write_all(&bytes)
            .map_err(|error| BuildError::io("write temporary build report", &temporary, error))?;
        file.sync_all()
            .map_err(|error| BuildError::io("sync temporary build report", &temporary, error))?;
    }
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| BuildError::io("replace build report", path, error))?;
    }
    fs::rename(&temporary, path)
        .map_err(|error| BuildError::io("replace build report", path, error))?;
    Ok(())
}

fn hash_file(path: &Path) -> BuildResult<String> {
    use std::io::Read;
    let mut file = fs::File::open(path)
        .map_err(|error| BuildError::io("open generated package", path, error))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)
            .map_err(|error| BuildError::io("hash generated package", path, error))?;
        if read == 0 { break; }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
