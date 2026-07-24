use std::path::PathBuf;

use vetrace_asset::{AssetId, AssetManager, AssetWatcher};
use vetrace_project::VetraceProject;

use crate::protocol::{AssetCacheSnapshot, AssetDiagnosticRow, AssetRow};

#[derive(Default)]
pub struct StudioAssets {
    manager: Option<AssetManager>,
    watcher: Option<AssetWatcher>,
    refresh_pending: bool,
    refresh_seconds: f32,
}

impl StudioAssets {
    pub fn initialize(&mut self, project: &VetraceProject) -> Vec<String> {
        let mut messages = Vec::new();
        match AssetManager::open(project) {
            Ok(mut manager) => {
                match manager.refresh() {
                    Ok(report) => messages.push(format!(
                        "Asset database ready: {} discovered, {} imported, {} diagnostics",
                        report.discovered, report.imported, report.diagnostics
                    )),
                    Err(error) => messages.push(format!("Initial asset refresh failed: {error}")),
                }
                match AssetWatcher::new(project.paths().assets()) {
                    Ok(watcher) => self.watcher = Some(watcher),
                    Err(error) => messages.push(format!("Asset file watching unavailable: {error}")),
                }
                self.manager = Some(manager);
            }
            Err(error) => messages.push(format!("Failed to open asset database: {error}")),
        }
        messages
    }

    pub fn update(&mut self, dt: f32) -> Vec<String> {
        let mut messages = Vec::new();
        if let Some(watcher) = &self.watcher {
            let changes = watcher.drain();
            messages.extend(changes.errors.into_iter().map(|error| format!("Asset watcher: {error}")));
            if changes.rescan_required || !changes.paths.is_empty() {
                self.refresh_pending = true;
                self.refresh_seconds = 0.0;
            }
        }
        if self.refresh_pending {
            self.refresh_seconds += dt.max(0.0).min(0.1);
            if self.refresh_seconds >= 0.25 {
                self.refresh_pending = false;
                self.refresh_seconds = 0.0;
                match self.refresh() {
                    Ok(status) | Err(status) => messages.push(status),
                }
            }
        }
        messages
    }

    pub fn refresh(&mut self) -> Result<String, String> {
        let manager = self.manager.as_mut().ok_or_else(|| "Asset database is unavailable".to_string())?;
        manager.refresh().map(|report| format!(
            "Assets: {} discovered, {} imported, {} diagnostics",
            report.discovered, report.imported, report.diagnostics
        )).map_err(|error| format!("Asset refresh failed: {error}"))
    }

    pub fn import_files(&mut self, paths: &[PathBuf]) -> Result<(String, Vec<String>), String> {
        let manager = self.manager.as_mut().ok_or_else(|| "Asset database is unavailable".to_string())?;
        let imported = manager.import_external_files(paths, None).map_err(|error| error.to_string())?;
        let logs = imported.iter().map(|file| {
            format!("Imported {} as {}", file.source.display(), file.destination)
        }).collect();
        Ok((format!("Imported {} asset files", imported.len()), logs))
    }

    pub fn reimport(&mut self, id: AssetId) -> Result<String, String> {
        let manager = self.manager.as_mut().ok_or_else(|| "Asset database is unavailable".to_string())?;
        manager.reimport(id).map(|()| format!("Reimported asset {id}")).map_err(|error| error.to_string())
    }

    pub fn reimport_all(&mut self) -> Result<String, String> {
        let manager = self.manager.as_mut().ok_or_else(|| "Asset database is unavailable".to_string())?;
        manager.reimport_all().map(|count| format!("Reimported {count} assets")).map_err(|error| error.to_string())
    }

    pub fn clear_cache(&mut self) -> Result<String, String> {
        {
            let manager = self.manager.as_mut().ok_or_else(|| "Asset database is unavailable".to_string())?;
            manager.clear_cache().map_err(|error| error.to_string())?;
        }
        self.refresh()?;
        Ok("Cleared and rebuilt imported asset cache".to_string())
    }

    pub fn prune_cache(&mut self) -> Result<String, String> {
        let manager = self.manager.as_mut().ok_or_else(|| "Asset database is unavailable".to_string())?;
        manager.prune_cache()
            .map(|count| format!("Removed {count} orphan cache directories"))
            .map_err(|error| error.to_string())
    }

    pub fn snapshot(&self) -> (Vec<AssetRow>, Vec<AssetDiagnosticRow>, AssetCacheSnapshot) {
        let Some(manager) = &self.manager else {
            return (Vec::new(), Vec::new(), AssetCacheSnapshot::default());
        };
        let mut rows: Vec<_> = manager.database().records.values().map(|record| AssetRow {
            id: record.id,
            path: record.source.to_string(),
            file_name: record.file_name().to_string(),
            kind: record.kind.clone(),
            status: record.status,
            size: record.source_size,
            hash: record.source_hash.clone(),
            importer: record.importer.as_ref().map(|stamp| format!("{} v{}", stamp.id, stamp.version)),
            outputs: record.outputs.iter().map(ToString::to_string).collect(),
            dependencies: record.dependencies.iter()
                .map(|dependency| (dependency.path.to_string(), dependency.missing))
                .collect(),
            error: record.last_error.clone(),
            thumbnail: record.outputs.iter()
                .find(|output| output.file_name() == Some("thumbnail.png"))
                .map(|output| manager.paths().resolve(output)),
        }).collect();
        rows.sort_by(|left, right| left.path.cmp(&right.path));
        let diagnostics = manager.database().diagnostics.iter().map(|diagnostic| AssetDiagnosticRow {
            severity: diagnostic.severity,
            code: diagnostic.code.clone(),
            message: diagnostic.message.clone(),
        }).collect();
        let stats = manager.cache_stats();
        let cache = AssetCacheSnapshot {
            imported_assets: stats.imported_assets,
            imported_files: stats.imported_files,
            bytes: stats.bytes,
            orphan_directories: stats.orphan_directories,
        };
        (rows, diagnostics, cache)
    }
}
