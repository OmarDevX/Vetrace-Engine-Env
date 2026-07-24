use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use vetrace_project::{ProjectPath, ProjectPaths, VetraceProject};

use crate::filesystem::{clear_directory, directory_size, now_unix_ms, unique_destination, write_json_atomic};
use crate::scan::{discover_files, DiscoveredFile};

use crate::{
    register_builtin_importers, AssetDatabase, AssetDependency, AssetDiagnostic,
    AssetDiagnosticSeverity, AssetError, AssetId, AssetKind, AssetRecord, AssetResult,
    AssetStatus, ImportContext, ImporterRegistry, ImporterStamp,
};

#[derive(Clone, Debug, Default)]
pub struct AssetRefreshReport {
    pub discovered: usize,
    pub added: usize,
    pub changed: usize,
    pub renamed: usize,
    pub missing: usize,
    pub imported: usize,
    pub failed: usize,
    pub unsupported: usize,
    pub diagnostics: usize,
}

#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    pub imported_assets: usize,
    pub imported_files: usize,
    pub bytes: u64,
    pub orphan_directories: usize,
}

#[derive(Clone, Debug)]
pub struct ImportedExternalFile {
    pub source: PathBuf,
    pub destination: ProjectPath,
}

pub struct AssetManager {
    paths: ProjectPaths,
    database: AssetDatabase,
    registry: ImporterRegistry,
}

impl AssetManager {
    pub fn open(project: &VetraceProject) -> AssetResult<Self> {
        project.paths().ensure_layout().map_err(|error| AssetError::Database(error.to_string()))?;
        let mut registry = ImporterRegistry::new();
        register_builtin_importers(&mut registry);
        let database = AssetDatabase::load(project.paths())?;
        Ok(Self { paths: project.paths().clone(), database, registry })
    }

    pub fn with_registry(project: &VetraceProject, registry: ImporterRegistry) -> AssetResult<Self> {
        project.paths().ensure_layout().map_err(|error| AssetError::Database(error.to_string()))?;
        let database = AssetDatabase::load(project.paths())?;
        Ok(Self { paths: project.paths().clone(), database, registry })
    }

    pub fn paths(&self) -> &ProjectPaths { &self.paths }
    pub fn database(&self) -> &AssetDatabase { &self.database }
    pub fn database_mut(&mut self) -> &mut AssetDatabase { &mut self.database }
    pub fn registry(&self) -> &ImporterRegistry { &self.registry }
    pub fn registry_mut(&mut self) -> &mut ImporterRegistry { &mut self.registry }

    pub fn asset_id(&self, source: &ProjectPath) -> Option<AssetId> {
        self.database.id_by_source(source)
    }

    pub fn source_path(&self, id: AssetId) -> Option<PathBuf> {
        self.database.record(id).map(|record| self.paths.resolve(&record.source))
    }

    pub fn imported_outputs(&self, id: AssetId) -> Vec<PathBuf> {
        self.database.record(id).map(|record| {
            record.outputs.iter().map(|output| self.paths.resolve(output)).collect()
        }).unwrap_or_default()
    }

    pub fn refresh(&mut self) -> AssetResult<AssetRefreshReport> {
        let discovered = discover_files(&self.paths)?;
        let mut report = self.reconcile(discovered)?;
        let ids: Vec<_> = self.database.records.values()
            .filter(|record| record.status == AssetStatus::NeedsImport)
            .map(|record| record.id)
            .collect();
        for id in ids {
            match self.import(id) {
                Ok(()) => report.imported += 1,
                Err(_) => report.failed += 1,
            }
        }
        self.resolve_dependencies();
        self.rebuild_diagnostics();
        report.diagnostics = self.database.diagnostics.len();
        self.database.save(&self.paths)?;
        Ok(report)
    }

    pub fn reimport(&mut self, id: AssetId) -> AssetResult<()> {
        let record = self.database.record_mut(id)
            .ok_or_else(|| AssetError::UnknownAsset(id.to_string()))?;
        if record.status == AssetStatus::Missing {
            return Err(AssetError::UnknownAsset(format!("{} is missing", record.source)));
        }
        record.status = AssetStatus::NeedsImport;
        record.last_error = None;
        self.import(id)?;
        self.resolve_dependencies();
        self.rebuild_diagnostics();
        self.database.save(&self.paths)
    }

    pub fn reimport_all(&mut self) -> AssetResult<usize> {
        let ids: Vec<_> = self.database.records.values()
            .filter(|record| record.status != AssetStatus::Missing)
            .map(|record| record.id)
            .collect();
        let mut imported = 0;
        for id in ids {
            if let Some(record) = self.database.record_mut(id) {
                record.status = AssetStatus::NeedsImport;
                record.last_error = None;
            }
            if self.import(id).is_ok() { imported += 1; }
        }
        self.resolve_dependencies();
        self.rebuild_diagnostics();
        self.database.save(&self.paths)?;
        Ok(imported)
    }

    pub fn import_external_files(
        &mut self,
        sources: &[PathBuf],
        destination_directory: Option<&ProjectPath>,
    ) -> AssetResult<Vec<ImportedExternalFile>> {
        if destination_directory.is_some_and(|path| !path.starts_with("assets")) {
            return Err(AssetError::InvalidPath("external assets must be copied under assets/".into()));
        }
        let canonical_assets = fs::canonicalize(self.paths.assets()).ok();
        let mut imported = Vec::new();
        for source in sources {
            if !source.is_file() { continue; }
            let canonical_source = fs::canonicalize(source)
                .map_err(|error| AssetError::io("canonicalize external asset", source, error))?;
            if canonical_assets.as_ref().is_some_and(|assets| canonical_source.starts_with(assets)) {
                let project_path = self.paths.to_project_path(&canonical_source)
                    .map_err(|error| AssetError::InvalidPath(error.to_string()))?;
                imported.push(ImportedExternalFile { source: source.clone(), destination: project_path });
                continue;
            }

            let destination = destination_directory.cloned()
                .unwrap_or_else(|| self.default_import_directory(source));
            let destination_absolute = self.paths.resolve_for_write(&destination)
                .map_err(|error| AssetError::InvalidPath(error.to_string()))?;
            fs::create_dir_all(&destination_absolute)
                .map_err(|error| AssetError::io("create asset import directory", &destination_absolute, error))?;
            let Some(file_name) = source.file_name() else { continue; };
            let target = unique_destination(&destination_absolute, file_name);
            fs::copy(source, &target)
                .map_err(|error| AssetError::io("copy external asset", &target, error))?;
            let project_path = self.paths.to_project_path(&target)
                .map_err(|error| AssetError::InvalidPath(error.to_string()))?;
            imported.push(ImportedExternalFile { source: source.clone(), destination: project_path });
        }
        self.refresh()?;
        Ok(imported)
    }

    fn default_import_directory(&self, source: &Path) -> ProjectPath {
        let value = self.registry.importer_for_path(source).map(|importer| match importer.kind() {
            AssetKind::Scene => "assets/scenes",
            AssetKind::Script => "assets/scripts",
            AssetKind::Model => "assets/models",
            AssetKind::Texture => "assets/textures",
            AssetKind::Audio => "assets/audio",
            AssetKind::Font => "assets/fonts",
            AssetKind::Shader => "assets/shaders",
            AssetKind::Material => "assets/materials",
            AssetKind::Data | AssetKind::Custom(_) | AssetKind::Unknown => "assets",
        }).unwrap_or("assets");
        ProjectPath::new(value).expect("built-in asset destination is valid")
    }

    pub fn clear_cache(&mut self) -> AssetResult<()> {
        clear_directory(self.paths.imported())?;
        fs::create_dir_all(self.paths.imported())
            .map_err(|error| AssetError::io("create imported cache", self.paths.imported(), error))?;
        for record in self.database.records.values_mut() {
            if record.status != AssetStatus::Missing && record.status != AssetStatus::Unsupported {
                record.status = AssetStatus::NeedsImport;
                record.outputs.clear();
                record.imported_unix_ms = None;
            }
        }
        self.database.save(&self.paths)
    }

    pub fn prune_cache(&mut self) -> AssetResult<usize> {
        let keep: BTreeSet<String> = self.database.records.keys().map(ToString::to_string).collect();
        let mut removed = 0;
        let entries = match fs::read_dir(self.paths.imported()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(AssetError::io("read imported cache", self.paths.imported(), error)),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if !keep.contains(&name) {
                fs::remove_dir_all(&path)
                    .map_err(|error| AssetError::io("remove orphan asset cache", &path, error))?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub fn cache_stats(&self) -> CacheStats {
        let mut stats = CacheStats::default();
        let keep: BTreeSet<String> = self.database.records.keys().map(ToString::to_string).collect();
        let Ok(entries) = fs::read_dir(self.paths.imported()) else { return stats; };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();
            if !keep.contains(&name) { stats.orphan_directories += 1; }
            let (files, bytes) = directory_size(&path);
            stats.imported_files += files;
            stats.bytes += bytes;
        }
        stats.imported_assets = self.database.records.values()
            .filter(|record| record.status == AssetStatus::Ready)
            .count();
        stats
    }

    fn reconcile(&mut self, discovered: Vec<DiscoveredFile>) -> AssetResult<AssetRefreshReport> {
        let mut report = AssetRefreshReport { discovered: discovered.len(), ..Default::default() };
        let old = self.database.records.clone();
        let current_sources: BTreeSet<_> = discovered.iter().map(|file| file.source.clone()).collect();
        let mut unused_old: BTreeSet<_> = old.keys().copied().collect();
        let mut records = BTreeMap::new();

        for file in discovered {
            let exact = old.values().find(|record| record.source == file.source).map(|record| record.id);
            let (id, renamed) = if let Some(id) = exact {
                (id, false)
            } else {
                let candidates: Vec<_> = old.values()
                    .filter(|record| {
                        unused_old.contains(&record.id)
                            && !current_sources.contains(&record.source)
                            && record.source_hash == file.hash
                            && record.source_size == file.size
                    })
                    .map(|record| record.id)
                    .collect();
                if candidates.len() == 1 { (candidates[0], true) } else { (AssetId::new(), false) }
            };
            unused_old.remove(&id);
            let previous = old.get(&id);
            let importer = self.registry.importer_for_path(&file.absolute);
            let kind = importer.as_ref().map(|value| value.kind()).unwrap_or(AssetKind::Unknown);
            let stamp = importer.as_ref().map(|value| ImporterStamp {
                id: value.id().to_string(), version: value.version(),
            });
            let unchanged = previous.is_some_and(|record| {
                record.source_hash == file.hash
                    && record.importer == stamp
                    && record.status == AssetStatus::Ready
                    && record.outputs.iter().all(|output| self.paths.resolve(output).exists())
            });
            let status = if importer.is_none() {
                report.unsupported += 1;
                AssetStatus::Unsupported
            } else if unchanged {
                AssetStatus::Ready
            } else {
                if previous.is_some() { report.changed += 1; } else { report.added += 1; }
                AssetStatus::NeedsImport
            };
            if renamed { report.renamed += 1; }
            records.insert(id, AssetRecord {
                id,
                source: file.source,
                kind,
                status,
                source_hash: file.hash,
                source_size: file.size,
                modified_unix_ms: file.modified_unix_ms,
                importer: stamp,
                outputs: previous.map(|record| record.outputs.clone()).unwrap_or_default(),
                dependencies: previous.map(|record| record.dependencies.clone()).unwrap_or_default(),
                imported_unix_ms: previous.and_then(|record| record.imported_unix_ms),
                last_error: if status == AssetStatus::Unsupported {
                    Some("no importer registered for this extension".into())
                } else { None },
            });
        }

        for id in unused_old {
            let Some(mut record) = old.get(&id).cloned() else { continue; };
            record.status = AssetStatus::Missing;
            record.last_error = Some("source file is missing".into());
            records.insert(id, record);
            report.missing += 1;
        }
        self.database.records = records;
        Ok(report)
    }

    fn import(&mut self, id: AssetId) -> AssetResult<()> {
        let record = self.database.record(id).cloned()
            .ok_or_else(|| AssetError::UnknownAsset(id.to_string()))?;
        let source_path = self.paths.resolve_existing(&record.source)
            .map_err(|error| AssetError::InvalidPath(error.to_string()))?;
        let importer = self.registry.importer_for_path(&source_path)
            .ok_or_else(|| AssetError::UnsupportedAsset(record.source.to_string()))?;
        let output_directory = self.paths.imported().join(id.to_string());
        if output_directory.exists() {
            fs::remove_dir_all(&output_directory)
                .map_err(|error| AssetError::io("clear imported asset directory", &output_directory, error))?;
        }
        fs::create_dir_all(&output_directory)
            .map_err(|error| AssetError::io("create imported asset directory", &output_directory, error))?;
        let context = ImportContext {
            id,
            source: &record.source,
            source_path: &source_path,
            project_paths: &self.paths,
            output_directory: &output_directory,
        };
        let result = importer.import(&context);
        match result {
            Ok(output) => {
                let outputs = output.outputs.iter()
                    .map(|path| self.paths.to_project_path(path)
                        .map_err(|error| AssetError::InvalidPath(error.to_string())))
                    .collect::<AssetResult<Vec<_>>>()?;
                let metadata_path = output_directory.join("import.json");
                let metadata = ImportMetadataFile {
                    asset_id: id,
                    source: record.source.clone(),
                    importer: ImporterStamp { id: importer.id().to_string(), version: importer.version() },
                    source_hash: record.source_hash.clone(),
                    outputs: outputs.clone(),
                    dependencies: output.dependencies.clone(),
                    metadata: output.metadata,
                    imported_unix_ms: now_unix_ms(),
                };
                write_json_atomic(&metadata_path, &metadata)?;
                let current = self.database.record_mut(id)
                    .ok_or_else(|| AssetError::UnknownAsset(id.to_string()))?;
                current.kind = importer.kind();
                current.status = AssetStatus::Ready;
                current.importer = Some(metadata.importer);
                current.outputs = outputs;
                current.dependencies = metadata.dependencies.into_iter().map(|path| AssetDependency {
                    path, asset_id: None, missing: false,
                }).collect();
                current.imported_unix_ms = Some(metadata.imported_unix_ms);
                current.last_error = None;
                Ok(())
            }
            Err(error) => {
                if let Some(current) = self.database.record_mut(id) {
                    current.status = AssetStatus::Failed;
                    current.outputs.clear();
                    current.imported_unix_ms = None;
                    current.last_error = Some(error.to_string());
                }
                Err(error)
            }
        }
    }

    fn resolve_dependencies(&mut self) {
        let source_ids: BTreeMap<_, _> = self.database.records.values()
            .map(|record| (record.source.clone(), record.id))
            .collect();
        let statuses: BTreeMap<_, _> = self.database.records.values()
            .map(|record| (record.id, record.status))
            .collect();
        for record in self.database.records.values_mut() {
            for dependency in &mut record.dependencies {
                dependency.asset_id = source_ids.get(&dependency.path).copied();
                dependency.missing = dependency.asset_id.is_none()
                    || dependency.asset_id.and_then(|id| statuses.get(&id).copied())
                        .is_some_and(|status| status == AssetStatus::Missing);
            }
        }
    }

    fn rebuild_diagnostics(&mut self) {
        let mut diagnostics = Vec::new();
        for record in self.database.records.values() {
            match record.status {
                AssetStatus::Missing => diagnostics.push(AssetDiagnostic {
                    severity: AssetDiagnosticSeverity::Error,
                    code: "asset.source_missing".into(),
                    asset_id: Some(record.id),
                    source: Some(record.source.clone()),
                    dependency: None,
                    message: format!("Asset source '{}' is missing", record.source),
                }),
                AssetStatus::Failed => diagnostics.push(AssetDiagnostic {
                    severity: AssetDiagnosticSeverity::Error,
                    code: "asset.import_failed".into(),
                    asset_id: Some(record.id),
                    source: Some(record.source.clone()),
                    dependency: None,
                    message: record.last_error.clone().unwrap_or_else(|| "Asset import failed".into()),
                }),
                AssetStatus::Unsupported => diagnostics.push(AssetDiagnostic {
                    severity: AssetDiagnosticSeverity::Warning,
                    code: "asset.unsupported".into(),
                    asset_id: Some(record.id),
                    source: Some(record.source.clone()),
                    dependency: None,
                    message: format!("No importer is registered for '{}'", record.source),
                }),
                _ => {}
            }
            for dependency in &record.dependencies {
                if dependency.missing {
                    diagnostics.push(AssetDiagnostic {
                        severity: AssetDiagnosticSeverity::Error,
                        code: "asset.dependency_missing".into(),
                        asset_id: Some(record.id),
                        source: Some(record.source.clone()),
                        dependency: Some(dependency.path.clone()),
                        message: format!("'{}' depends on missing asset '{}'", record.source, dependency.path),
                    });
                }
            }
        }
        self.database.diagnostics = diagnostics;
    }
}

#[derive(Serialize)]
struct ImportMetadataFile {
    asset_id: AssetId,
    source: ProjectPath,
    importer: ImporterStamp,
    source_hash: String,
    outputs: Vec<ProjectPath>,
    dependencies: Vec<ProjectPath>,
    metadata: BTreeMap<String, String>,
    imported_unix_ms: u64,
}
