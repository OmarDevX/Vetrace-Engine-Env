use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vetrace_project::{VetraceProject, PROJECT_MANIFEST_FILE};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{BuildError, BuildResult, CompressionMode};

pub const VPAK_FORMAT_VERSION: u32 = 1;
const ASSET_DATABASE_PATH: &str = ".vetrace/asset_db.json";
pub const VPAK_MANIFEST_FILE: &str = "vpak.json";
const MAX_PACKAGE_ENTRIES: usize = 1_000_000;
const MAX_PACKAGE_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PACKAGE_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    pub path: String,
    pub size: u64,
    pub blake3: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub format_version: u32,
    pub project_id: Uuid,
    pub project_name: String,
    pub project_version: String,
    pub engine_version: String,
    pub created_unix_ms: u64,
    pub entries: Vec<PackageEntry>,
}

#[derive(Clone, Copy, Debug)]
pub struct PackageOptions {
    pub compression: CompressionMode,
    pub include_asset_database: bool,
}

impl Default for PackageOptions {
    fn default() -> Self {
        Self { compression: CompressionMode::Deflate, include_asset_database: true }
    }
}

pub fn create_vpak(
    project: &VetraceProject,
    destination: impl AsRef<Path>,
    options: PackageOptions,
) -> BuildResult<PackageManifest> {
    let report = project.validate_files();
    if !report.is_valid() {
        return Err(BuildError::Project(vetrace_project::ProjectError::Validation(report)));
    }

    let destination = destination.as_ref();
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| BuildError::io("create package directory", parent, error))?;
    }
    let temporary = destination.with_extension("vpak.tmp");
    let file = fs::File::create(&temporary)
        .map_err(|error| BuildError::io("create temporary package", &temporary, error))?;
    let mut writer = ZipWriter::new(file);
    let compression = match options.compression {
        CompressionMode::Stored => CompressionMethod::Stored,
        CompressionMode::Deflate => CompressionMethod::Deflated,
    };
    let file_options = SimpleFileOptions::default()
        .compression_method(compression)
        .unix_permissions(0o644);

    let mut source_entries = Vec::new();
    source_entries.push((
        PROJECT_MANIFEST_FILE.to_owned(),
        project.paths().manifest().to_path_buf(),
    ));
    collect_files(project.paths().assets(), project.root(), &mut source_entries)?;
    if options.include_asset_database {
        let database = project.root().join(ASSET_DATABASE_PATH);
        if database.is_file() {
            source_entries.push((ASSET_DATABASE_PATH.to_owned(), database));
        }
    }
    source_entries.sort_by(|left, right| left.0.cmp(&right.0));
    source_entries.dedup_by(|left, right| left.0 == right.0);

    let mut entries = Vec::with_capacity(source_entries.len());
    for (archive_path, source_path) in source_entries {
        validate_archive_path(&archive_path)?;
        let mut source = fs::File::open(&source_path)
            .map_err(|error| BuildError::io("open package input", &source_path, error))?;
        let size = source.metadata()
            .map_err(|error| BuildError::io("inspect package input", &source_path, error))?
            .len();
        let options = file_options.clone().large_file(size >= zip::ZIP64_BYTES_THR);
        writer.start_file(&archive_path, options)?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = source.read(&mut buffer)
                .map_err(|error| BuildError::io("read package input", &source_path, error))?;
            if read == 0 { break; }
            hasher.update(&buffer[..read]);
            writer.write_all(&buffer[..read])
                .map_err(|error| BuildError::io("write package entry", &temporary, error))?;
        }
        entries.push(PackageEntry {
            path: archive_path,
            size,
            blake3: hasher.finalize().to_hex().to_string(),
        });
    }

    let manifest = PackageManifest {
        format_version: VPAK_FORMAT_VERSION,
        project_id: project.manifest().project.id,
        project_name: project.manifest().project.name.clone(),
        project_version: project.manifest().project.version.clone(),
        engine_version: project.manifest().project.engine_version.clone(),
        created_unix_ms: now_unix_ms(),
        entries,
    };
    let metadata = serde_json::to_vec_pretty(&manifest)?;
    writer.start_file(VPAK_MANIFEST_FILE, file_options.clone())?;
    writer.write_all(&metadata)
        .map_err(|error| BuildError::io("write package manifest", &temporary, error))?;
    let mut file = writer.finish()?;
    file.flush().map_err(|error| BuildError::io("flush package", &temporary, error))?;
    file.sync_all().map_err(|error| BuildError::io("sync package", &temporary, error))?;
    drop(file);

    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| BuildError::io("replace package", destination, error))?;
    }
    fs::rename(&temporary, destination)
        .map_err(|error| BuildError::io("replace package", destination, error))?;
    Ok(manifest)
}

pub fn inspect_vpak(path: impl AsRef<Path>) -> BuildResult<PackageManifest> {
    let path = path.as_ref();
    let file = fs::File::open(path)
        .map_err(|error| BuildError::io("open package", path, error))?;
    let mut archive = ZipArchive::new(file)?;
    if archive.len() > MAX_PACKAGE_ENTRIES.saturating_add(1) {
        return Err(BuildError::InvalidPackage(format!(
            "package archive contains too many entries ({})",
            archive.len()
        )));
    }
    let mut metadata = archive.by_name(VPAK_MANIFEST_FILE)
        .map_err(|_| BuildError::InvalidPackage(format!("missing {VPAK_MANIFEST_FILE}")))?;
    if metadata.size() > MAX_PACKAGE_MANIFEST_BYTES {
        return Err(BuildError::InvalidPackage(
            "package manifest exceeds the 16 MiB safety limit".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.size() as usize);
    metadata.read_to_end(&mut bytes)
        .map_err(|error| BuildError::io("read package manifest", path, error))?;
    let manifest: PackageManifest = serde_json::from_slice(&bytes)?;
    if manifest.format_version != VPAK_FORMAT_VERSION {
        return Err(BuildError::InvalidPackage(format!(
            "unsupported format {}; expected {}",
            manifest.format_version, VPAK_FORMAT_VERSION
        )));
    }
    validate_manifest_entries(&manifest)?;
    Ok(manifest)
}

pub struct PackageMount {
    root: PathBuf,
    project: VetraceProject,
}

impl PackageMount {
    pub fn root(&self) -> &Path { &self.root }
    pub fn project(&self) -> &VetraceProject { &self.project }
}

impl Drop for PackageMount {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn mount_vpak(path: impl AsRef<Path>) -> BuildResult<PackageMount> {
    let package_path = path.as_ref();
    let manifest = inspect_vpak(package_path)?;
    let root = std::env::temp_dir().join(format!(
        "vetrace-vpak-{}-{}",
        manifest.project_id,
        Uuid::new_v4()
    ));
    fs::create_dir_all(&root)
        .map_err(|error| BuildError::io("create package mount", &root, error))?;
    if let Err(error) = extract_archive(package_path, &root, &manifest) {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }
    let project = match VetraceProject::load(&root) {
        Ok(project) => project,
        Err(error) => {
            let _ = fs::remove_dir_all(&root);
            return Err(error.into());
        }
    };
    if let Err(error) = validate_project_identity(&manifest, &project) {
        let _ = fs::remove_dir_all(&root);
        return Err(error);
    }
    Ok(PackageMount { root, project })
}

fn extract_archive(
    package_path: &Path,
    root: &Path,
    manifest: &PackageManifest,
) -> BuildResult<()> {
    let file = fs::File::open(package_path)
        .map_err(|error| BuildError::io("open package", package_path, error))?;
    let mut archive = ZipArchive::new(file)?;
    if archive.len() > MAX_PACKAGE_ENTRIES.saturating_add(1) {
        return Err(BuildError::InvalidPackage(format!(
            "package archive contains too many entries ({})",
            archive.len()
        )));
    }
    let expected: std::collections::BTreeMap<_, _> = manifest.entries.iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let mut extracted = BTreeSet::new();
    let mut package_manifest_count = 0_usize;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_owned();
        if name == VPAK_MANIFEST_FILE {
            package_manifest_count += 1;
            continue;
        }
        validate_archive_path(&name)?;
        if entry.is_dir() { continue; }
        let expected_entry = expected.get(name.as_str()).ok_or_else(|| {
            BuildError::InvalidPackage(format!("undeclared package entry '{name}'"))
        })?;
        if !extracted.insert(name.clone()) {
            return Err(BuildError::InvalidPackage(format!("duplicate package entry '{name}'")));
        }
        let output = root.join(Path::new(&name));
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| BuildError::io("create package output directory", parent, error))?;
        }
        if entry.size() != expected_entry.size {
            return Err(BuildError::InvalidPackage(format!(
                "declared size mismatch for '{name}'"
            )));
        }
        let mut output_file = fs::File::create(&output)
            .map_err(|error| BuildError::io("create extracted package entry", &output, error))?;
        let mut hasher = blake3::Hasher::new();
        let mut written = 0_u64;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = entry.read(&mut buffer)
                .map_err(|error| BuildError::io("read package entry", package_path, error))?;
            if read == 0 { break; }
            written = written.saturating_add(read as u64);
            if written > expected_entry.size {
                return Err(BuildError::InvalidPackage(format!(
                    "entry '{name}' exceeds its declared size"
                )));
            }
            hasher.update(&buffer[..read]);
            output_file.write_all(&buffer[..read])
                .map_err(|error| BuildError::io("extract package entry", &output, error))?;
        }
        output_file.flush()
            .map_err(|error| BuildError::io("flush extracted package entry", &output, error))?;
        if written != expected_entry.size
            || hasher.finalize().to_hex().to_string() != expected_entry.blake3
        {
            return Err(BuildError::InvalidPackage(format!(
                "integrity check failed for '{name}'"
            )));
        }
    }

    if package_manifest_count != 1 {
        return Err(BuildError::InvalidPackage(format!(
            "expected exactly one {VPAK_MANIFEST_FILE}, found {package_manifest_count}"
        )));
    }
    if extracted.len() != expected.len() {
        let missing = expected.keys()
            .copied()
            .find(|name| !extracted.contains(*name))
            .unwrap_or("unknown");
        return Err(BuildError::InvalidPackage(format!(
            "declared entry '{missing}' is missing"
        )));
    }
    Ok(())
}

fn validate_manifest_entries(manifest: &PackageManifest) -> BuildResult<()> {
    if manifest.entries.len() > MAX_PACKAGE_ENTRIES {
        return Err(BuildError::InvalidPackage(format!(
            "package declares too many entries ({})",
            manifest.entries.len()
        )));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0_u64;
    for entry in &manifest.entries {
        validate_archive_path(&entry.path)?;
        if entry.path != PROJECT_MANIFEST_FILE
            && !entry.path.starts_with("assets/")
            && entry.path != ASSET_DATABASE_PATH
        {
            return Err(BuildError::InvalidPackage(format!(
                "entry '{}' is outside allowed project package paths",
                entry.path
            )));
        }
        if !seen.insert(entry.path.as_str()) {
            return Err(BuildError::InvalidPackage(format!(
                "duplicate declared entry '{}'",
                entry.path
            )));
        }
        total = total.checked_add(entry.size).ok_or_else(|| {
            BuildError::InvalidPackage("package size overflow".to_owned())
        })?;
        if total > MAX_PACKAGE_UNCOMPRESSED_BYTES {
            return Err(BuildError::InvalidPackage(format!(
                "package exceeds the {} GiB uncompressed safety limit",
                MAX_PACKAGE_UNCOMPRESSED_BYTES / (1024 * 1024 * 1024)
            )));
        }
        if entry.blake3.len() != 64 || !entry.blake3.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(BuildError::InvalidPackage(format!(
                "entry '{}' has an invalid BLAKE3 digest",
                entry.path
            )));
        }
    }
    if !seen.contains(PROJECT_MANIFEST_FILE) {
        return Err(BuildError::InvalidPackage(format!(
            "missing declared {PROJECT_MANIFEST_FILE}"
        )));
    }
    Ok(())
}


fn validate_project_identity(
    package: &PackageManifest,
    project: &VetraceProject,
) -> BuildResult<()> {
    let project_manifest = project.manifest();
    let matches = package.project_id == project_manifest.project.id
        && package.project_name == project_manifest.project.name
        && package.project_version == project_manifest.project.version
        && package.engine_version == project_manifest.project.engine_version;
    if matches {
        return Ok(());
    }
    Err(BuildError::InvalidPackage(
        "package metadata does not match the embedded project manifest".to_owned(),
    ))
}

fn collect_files(
    directory: &Path,
    root: &Path,
    output: &mut Vec<(String, PathBuf)>,
) -> BuildResult<()> {
    let entries = fs::read_dir(directory)
        .map_err(|error| BuildError::io("read package input directory", directory, error))?;
    for entry in entries {
        let entry = entry
            .map_err(|error| BuildError::io("read package input entry", directory, error))?;
        let file_type = entry.file_type()
            .map_err(|error| BuildError::io("inspect package input", entry.path(), error))?;
        if file_type.is_symlink() { continue; }
        let path = entry.path();
        if file_type.is_dir() {
            collect_files(&path, root, output)?;
        } else if file_type.is_file() {
            let relative = path.strip_prefix(root)
                .map_err(|_| BuildError::InvalidPackage(format!(
                    "package input '{}' is outside project root",
                    path.display()
                )))?;
            let archive_path = relative.components().map(|component| {
                component.as_os_str().to_str().ok_or_else(|| {
                    BuildError::Validation(format!(
                        "asset path '{}' is not valid UTF-8 and cannot be packaged portably",
                        path.display()
                    ))
                })
            }).collect::<BuildResult<Vec<_>>>()?.join("/");
            output.push((archive_path, path));
        }
    }
    Ok(())
}

fn validate_archive_path(value: &str) -> BuildResult<()> {
    if value.is_empty()
        || value.contains('\\')
        || value.starts_with('/')
        || value.contains('\0')
        || value.split('/').any(|segment| matches!(segment, "" | "." | ".."))
    {
        return Err(BuildError::InvalidPackage(format!("unsafe entry path '{value}'")));
    }
    let path = Path::new(value);
    if path.components().any(|component| {
        matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
    }) {
        return Err(BuildError::InvalidPackage(format!("unsafe entry path '{value}'")));
    }
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
