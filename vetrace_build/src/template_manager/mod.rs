use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::{
    player_template_metadata_path, BuildError, BuildResult, PlayerTemplateMetadata,
    PlayerTemplateTarget, PLAYER_TEMPLATE_METADATA_FORMAT_VERSION, VPAK_FORMAT_VERSION,
};

pub const TEMPLATE_INDEX_FORMAT_VERSION: u32 = 1;
pub const TEMPLATE_CATALOG_FORMAT_VERSION: u32 = 1;
pub const TEMPLATE_BUNDLE_MANIFEST: &str = "vetrace-template.json";



#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTemplateCatalogEntry {
    pub id: String,
    pub engine_version: String,
    pub target: PlayerTemplateTarget,
    pub url: String,
    pub blake3: String,
    #[serde(default)]
    pub bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTemplateCatalog {
    pub format_version: u32,
    #[serde(default)]
    pub generated_unix_ms: u64,
    pub templates: Vec<PlayerTemplateCatalogEntry>,
}

impl PlayerTemplateCatalog {
    pub fn load(path: impl AsRef<Path>) -> BuildResult<Self> {
        let path = path.as_ref();
        let catalog: Self = serde_json::from_slice(
            &fs::read(path).map_err(|error| BuildError::io("read player-template catalog", path, error))?,
        )?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> BuildResult<()> {
        self.validate()?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| BuildError::io("create player-template catalog directory", parent, error))?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)
            .map_err(|error| BuildError::io("write player-template catalog", &temporary, error))?;
        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(path)
                .map_err(|error| BuildError::io("replace player-template catalog", path, error))?;
        }
        fs::rename(&temporary, path)
            .map_err(|error| BuildError::io("replace player-template catalog", path, error))
    }

    pub fn validate(&self) -> BuildResult<()> {
        if self.format_version != TEMPLATE_CATALOG_FORMAT_VERSION {
            return Err(BuildError::Validation(format!(
                "unsupported player-template catalog format {}; expected {}",
                self.format_version, TEMPLATE_CATALOG_FORMAT_VERSION,
            )));
        }
        let mut ids = std::collections::BTreeSet::new();
        for entry in &self.templates {
            if entry.id.trim().is_empty() || !ids.insert(entry.id.clone()) {
                return Err(BuildError::Validation(format!(
                    "player-template catalog contains an empty or duplicate id '{}'",
                    entry.id,
                )));
            }
            if entry.engine_version.trim().is_empty() {
                return Err(BuildError::Validation(format!(
                    "player-template catalog entry '{}' has no engine version",
                    entry.id,
                )));
            }
            validate_catalog_url(&entry.url).map_err(|_| BuildError::Validation(format!(
                "player-template catalog entry '{}' must use HTTPS (HTTP is allowed only for localhost)",
                entry.id,
            )))?;
            if entry.blake3.len() != 64 || !entry.blake3.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(BuildError::Validation(format!(
                    "player-template catalog entry '{}' has an invalid BLAKE3 digest",
                    entry.id,
                )));
            }
        }
        Ok(())
    }

    pub fn find(
        &self,
        target: PlayerTemplateTarget,
        engine_version: &str,
    ) -> Option<&PlayerTemplateCatalogEntry> {
        self.templates.iter().find(|entry| {
            entry.target == target && entry.engine_version == engine_version
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPlayerTemplate {
    pub id: String,
    pub engine_version: String,
    pub target: PlayerTemplateTarget,
    pub binary: PathBuf,
    pub metadata: PathBuf,
    pub installed_unix_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TemplateIndex {
    format_version: u32,
    templates: Vec<InstalledPlayerTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TemplateBundleManifest {
    format_version: u32,
    id: String,
    engine_version: String,
    target: PlayerTemplateTarget,
    binary: String,
    metadata: String,
}



pub fn create_template_bundle(
    id: impl Into<String>,
    player_binary: impl AsRef<Path>,
    destination: impl AsRef<Path>,
) -> BuildResult<PathBuf> {
    let id = sanitize_template_id(&id.into());
    let player_binary = player_binary.as_ref();
    let destination = destination.as_ref();
    if !player_binary.is_file() {
        return Err(BuildError::MissingPlayerTemplate(player_binary.to_path_buf()));
    }
    let metadata_path = player_template_metadata_path(player_binary);
    let metadata: PlayerTemplateMetadata = serde_json::from_slice(
        &fs::read(&metadata_path)
            .map_err(|error| BuildError::io("read player-template metadata", &metadata_path, error))?,
    )?;
    validate_template_metadata_shape(&metadata)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| BuildError::io("create template-bundle directory", parent, error))?;
    }
    let binary_name = player_binary.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BuildError::Validation("player-template file name is not valid UTF-8".into()))?
        .to_owned();
    let metadata_name = metadata_path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BuildError::Validation("player-template metadata name is not valid UTF-8".into()))?
        .to_owned();
    let manifest = TemplateBundleManifest {
        format_version: TEMPLATE_INDEX_FORMAT_VERSION,
        id,
        engine_version: metadata.engine_version.clone(),
        target: metadata.target,
        binary: binary_name.clone(),
        metadata: metadata_name.clone(),
    };
    let temporary = destination.with_extension("vtemplate.tmp");
    let file = fs::File::create(&temporary)
        .map_err(|error| BuildError::io("create player-template bundle", &temporary, error))?;
    let mut archive = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    archive.start_file(TEMPLATE_BUNDLE_MANIFEST, options)?;
    archive.write_all(&serde_json::to_vec_pretty(&manifest)?)
        .map_err(|error| BuildError::io("write template bundle manifest", &temporary, error))?;
    archive.start_file(binary_name, options)?;
    let mut binary_file = fs::File::open(player_binary)
        .map_err(|error| BuildError::io("open player-template binary", player_binary, error))?;
    std::io::copy(&mut binary_file, &mut archive)
        .map_err(|error| BuildError::io("write player-template binary", &temporary, error))?;
    archive.start_file(metadata_name, options)?;
    let mut metadata_file = fs::File::open(&metadata_path)
        .map_err(|error| BuildError::io("open player-template metadata", &metadata_path, error))?;
    std::io::copy(&mut metadata_file, &mut archive)
        .map_err(|error| BuildError::io("write player-template metadata", &temporary, error))?;
    archive.finish()?;
    #[cfg(windows)]
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| BuildError::io("replace player-template bundle", destination, error))?;
    }
    fs::rename(&temporary, destination)
        .map_err(|error| BuildError::io("replace player-template bundle", destination, error))?;
    Ok(destination.to_path_buf())
}

pub struct PlayerTemplateManager {
    root: PathBuf,
    index: TemplateIndex,
}

impl PlayerTemplateManager {
    pub fn open_default() -> BuildResult<Self> {
        Self::open(default_template_root())
    }

    pub fn open(root: impl AsRef<Path>) -> BuildResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)
            .map_err(|error| BuildError::io("create player-template directory", &root, error))?;
        let index_path = root.join("index.json");
        let mut index = match fs::read(&index_path) {
            Ok(bytes) => serde_json::from_slice::<TemplateIndex>(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => TemplateIndex {
                format_version: TEMPLATE_INDEX_FORMAT_VERSION,
                templates: Vec::new(),
            },
            Err(error) => return Err(BuildError::io("read player-template index", &index_path, error)),
        };
        if index.format_version == 0 { index.format_version = TEMPLATE_INDEX_FORMAT_VERSION; }
        if index.format_version != TEMPLATE_INDEX_FORMAT_VERSION {
            return Err(BuildError::Validation(format!(
                "unsupported player-template index format {}; expected {}",
                index.format_version, TEMPLATE_INDEX_FORMAT_VERSION
            )));
        }
        index.templates.retain(|template| template.binary.is_file() && template.metadata.is_file());
        let manager = Self { root, index };
        manager.save_index()?;
        Ok(manager)
    }

    pub fn root(&self) -> &Path { &self.root }
    pub fn templates(&self) -> &[InstalledPlayerTemplate] { &self.index.templates }

    pub fn find(
        &self,
        target: PlayerTemplateTarget,
        engine_version: &str,
    ) -> Option<&InstalledPlayerTemplate> {
        self.index.templates.iter().find(|template| {
            template.target == target && template.engine_version == engine_version
        })
    }

    pub fn install_binary(
        &mut self,
        id: impl Into<String>,
        player_binary: impl AsRef<Path>,
        metadata: &PlayerTemplateMetadata,
    ) -> BuildResult<InstalledPlayerTemplate> {
        let id = sanitize_template_id(&id.into());
        let source = player_binary.as_ref();
        if !source.is_file() { return Err(BuildError::MissingPlayerTemplate(source.to_path_buf())); }
        validate_template_metadata_shape(metadata)?;
        let directory = self.root.join(&id);
        if directory.exists() {
            fs::remove_dir_all(&directory)
                .map_err(|error| BuildError::io("replace installed player template", &directory, error))?;
        }
        fs::create_dir_all(&directory)
            .map_err(|error| BuildError::io("create installed player template", &directory, error))?;
        let file_name = source.file_name().ok_or_else(|| {
            BuildError::Validation("player template has no file name".into())
        })?;
        let binary = directory.join(file_name);
        fs::copy(source, &binary)
            .map_err(|error| BuildError::io("install player template", &binary, error))?;
        copy_executable_permissions(source, &binary)?;
        let metadata_path = player_template_metadata_path(&binary);
        fs::write(&metadata_path, serde_json::to_vec_pretty(metadata)?)
            .map_err(|error| BuildError::io("write installed template metadata", &metadata_path, error))?;
        let installed = InstalledPlayerTemplate {
            id: id.clone(),
            engine_version: metadata.engine_version.clone(),
            target: metadata.target,
            binary,
            metadata: metadata_path,
            installed_unix_ms: now_unix_ms(),
        };
        self.index.templates.retain(|template| template.id != id);
        self.index.templates.push(installed.clone());
        self.index.templates.sort_by(|left, right| left.id.cmp(&right.id));
        self.save_index()?;
        Ok(installed)
    }

    pub fn install_archive(&mut self, archive_path: impl AsRef<Path>) -> BuildResult<InstalledPlayerTemplate> {
        let archive_path = archive_path.as_ref();
        let file = fs::File::open(archive_path)
            .map_err(|error| BuildError::io("open player-template bundle", archive_path, error))?;
        let mut archive = ZipArchive::new(file)?;
        let manifest: TemplateBundleManifest = {
            let mut entry = archive.by_name(TEMPLATE_BUNDLE_MANIFEST).map_err(|_| {
                BuildError::Validation(format!("template bundle is missing {TEMPLATE_BUNDLE_MANIFEST}"))
            })?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)
                .map_err(|error| BuildError::io("read template bundle manifest", archive_path, error))?;
            serde_json::from_slice(&bytes)?
        };
        if manifest.format_version != TEMPLATE_INDEX_FORMAT_VERSION {
            return Err(BuildError::Validation(format!(
                "unsupported player-template bundle format {}",
                manifest.format_version
            )));
        }
        let staging = self.root.join(format!(".{}.installing", sanitize_template_id(&manifest.id)));
        if staging.exists() {
            fs::remove_dir_all(&staging)
                .map_err(|error| BuildError::io("clear template staging directory", &staging, error))?;
        }
        fs::create_dir_all(&staging)
            .map_err(|error| BuildError::io("create template staging directory", &staging, error))?;
        let result = (|| -> BuildResult<InstalledPlayerTemplate> {
            extract_safe(&mut archive, &staging)?;
            let binary_source = safe_join(&staging, &manifest.binary)?;
            let metadata_source = safe_join(&staging, &manifest.metadata)?;
            let metadata: PlayerTemplateMetadata = serde_json::from_slice(
                &fs::read(&metadata_source)
                    .map_err(|error| BuildError::io("read bundled template metadata", &metadata_source, error))?
            )?;
            validate_template_metadata_shape(&metadata)?;
            if metadata.engine_version != manifest.engine_version || metadata.target != manifest.target {
                return Err(BuildError::Validation(
                    "template bundle manifest does not match player metadata".into(),
                ));
            }
            self.install_binary(manifest.id, binary_source, &metadata)
        })();
        let _ = fs::remove_dir_all(&staging);
        result
    }

    #[cfg(feature = "template_download")]
    pub fn download_and_install(&mut self, url: &str) -> BuildResult<InstalledPlayerTemplate> {
        let temporary = self.root.join(".download.vtemplate");
        download_to_file(url, &temporary, None)?;
        let result = self.install_archive(&temporary);
        let _ = fs::remove_file(&temporary);
        result
    }

    #[cfg(feature = "template_download")]
    pub fn download_catalog(url: &str) -> BuildResult<PlayerTemplateCatalog> {
        validate_http_url(url)?;
        let temporary = std::env::temp_dir().join(format!(
            "vetrace-template-catalog-{}-{}.json",
            std::process::id(),
            uuid::Uuid::new_v4(),
        ));
        download_to_file(url, &temporary, None)?;
        let result = PlayerTemplateCatalog::load(&temporary);
        let _ = fs::remove_file(&temporary);
        result
    }

    #[cfg(feature = "template_download")]
    pub fn download_catalog_entry(
        &mut self,
        catalog: &PlayerTemplateCatalog,
        entry_id: &str,
    ) -> BuildResult<InstalledPlayerTemplate> {
        catalog.validate()?;
        let entry = catalog.templates.iter().find(|entry| entry.id == entry_id).ok_or_else(|| {
            BuildError::Validation(format!("player-template catalog has no entry '{entry_id}'"))
        })?;
        let temporary = self.root.join(format!(".{}.download.vtemplate", sanitize_template_id(entry_id)));
        download_to_file(&entry.url, &temporary, entry.bytes)?;
        let digest = blake3_file(&temporary)?;
        if !digest.eq_ignore_ascii_case(&entry.blake3) {
            let _ = fs::remove_file(&temporary);
            return Err(BuildError::Validation(format!(
                "downloaded player-template '{}' failed BLAKE3 verification",
                entry.id,
            )));
        }
        let result = self.install_archive(&temporary);
        let _ = fs::remove_file(&temporary);
        let installed = result?;
        if installed.id != entry.id
            || installed.engine_version != entry.engine_version
            || installed.target != entry.target
        {
            let _ = self.remove(&installed.id);
            return Err(BuildError::Validation(format!(
                "downloaded player-template '{}' does not match its catalog entry",
                entry.id,
            )));
        }
        Ok(installed)
    }

    pub fn remove(&mut self, id: &str) -> BuildResult<bool> {
        let Some(index) = self.index.templates.iter().position(|template| template.id == id) else {
            return Ok(false);
        };
        let template = self.index.templates.remove(index);
        if let Some(directory) = template.binary.parent() {
            if directory.starts_with(&self.root) && directory != self.root {
                fs::remove_dir_all(directory)
                    .map_err(|error| BuildError::io("remove player template", directory, error))?;
            }
        }
        self.save_index()?;
        Ok(true)
    }

    fn save_index(&self) -> BuildResult<()> {
        let path = self.root.join("index.json");
        let temporary = self.root.join(".index.json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&self.index)?)
            .map_err(|error| BuildError::io("write player-template index", &temporary, error))?;
        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|error| BuildError::io("replace player-template index", &path, error))?;
        }
        fs::rename(&temporary, &path)
            .map_err(|error| BuildError::io("replace player-template index", &path, error))
    }
}


mod helpers;

pub use helpers::default_template_root;
use helpers::*;
