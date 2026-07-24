use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use vetrace_build::{
    build_project, find_player_template, load_player_template_metadata, BuildAssetPreflight,
    BuildReport, BuildRequest, ExportConfig, ExportPreset, PlayerTemplateManager,
    PlayerTemplateTarget,
};
use vetrace_project::VetraceProject;

use crate::protocol::{BuildReportSnapshot, BuildSnapshot, PlayerTemplateSnapshot};

pub struct StudioBuilds {
    config: ExportConfig,
    detected_template: Option<PathBuf>,
    task: Option<BuildTask>,
    status: String,
    last_report: Option<BuildReport>,
    template_manager: Option<PlayerTemplateManager>,
    project_engine_version: String,
}

struct BuildTask {
    receiver: Receiver<Result<BuildReport, String>>,
}

impl StudioBuilds {
    pub fn initialize(project: &VetraceProject) -> (Self, Vec<String>) {
        let mut logs = Vec::new();
        let config = match ExportConfig::load_or_default(project) {
            Ok(config) => config,
            Err(error) => {
                logs.push(format!("Failed to load export presets: {error}"));
                ExportConfig::default()
            }
        };
        if !ExportConfig::path(project).is_file() {
            if let Err(error) = config.save(project) {
                logs.push(format!("Failed to create default export preset: {error}"));
            }
        }
        let mut template_manager = match PlayerTemplateManager::open_default() {
            Ok(manager) => Some(manager),
            Err(error) => {
                logs.push(format!("Failed to open player-template registry: {error}"));
                None
            }
        };
        let project_engine_version = project.manifest().project.engine_version.clone();
        let managed = PlayerTemplateTarget::current().and_then(|target| {
            template_manager.as_ref()
                .and_then(|manager| manager.find(target, &project_engine_version))
                .map(|template| template.binary.clone())
        });
        let detected_template = find_player_template().or(managed);
        if detected_template.is_none() {
            logs.push(
                "No compatible vetrace-player template was detected. Install a template bundle or browse to a prebuilt binary in the Build tab."
                    .to_owned(),
            );
        }
        (
            Self {
                config,
                detected_template,
                task: None,
                status: "Ready to export".to_owned(),
                last_report: None,
                template_manager,
                project_engine_version,
            },
            logs,
        )
    }

    pub fn snapshot(&self) -> BuildSnapshot {
        BuildSnapshot {
            presets: self.config.presets.clone(),
            default_preset: self.config.default_preset.clone(),
            detected_player_template: self.detected_template.clone(),
            installed_templates: self.template_manager.as_ref().map(|manager| {
                manager.templates().iter().map(|template| PlayerTemplateSnapshot {
                    id: template.id.clone(),
                    engine_version: template.engine_version.clone(),
                    target: format!("{:?}", template.target),
                    binary: template.binary.clone(),
                }).collect()
            }).unwrap_or_default(),
            running: self.task.is_some(),
            status: self.status.clone(),
            last_report: self.last_report.as_ref().map(|report| BuildReportSnapshot {
                output_directory: report.output_directory.clone(),
                executable: report.executable.clone(),
                package: report.package.clone(),
                package_entries: report.package_entries,
                package_bytes: report.package_bytes,
                package_blake3: report.package_blake3.clone(),
                warnings: report.warnings.clone(),
            }),
        }
    }

    pub fn save_preset(
        &mut self,
        project: &VetraceProject,
        preset: ExportPreset,
    ) -> Result<String, String> {
        self.config.upsert(preset.clone()).map_err(|error| error.to_string())?;
        self.config.default_preset = preset.name.clone();
        self.config.save(project).map_err(|error| error.to_string())?;
        self.status = format!("Saved export preset '{}'", preset.name);
        Ok(self.status.clone())
    }

    pub fn install_template_binary(&mut self, path: &Path) -> Result<String, String> {
        let metadata = load_player_template_metadata(path).map_err(|error| error.to_string())?;
        let id = format!("{}-{:?}", metadata.engine_version, metadata.target).to_ascii_lowercase();
        let manager = self.template_manager.as_mut().ok_or_else(|| "player-template registry is unavailable".to_owned())?;
        let installed = manager.install_binary(id, path, &metadata).map_err(|error| error.to_string())?;
        self.detected_template = Some(installed.binary.clone());
        self.status = format!("Installed player template '{}'", installed.id);
        Ok(self.status.clone())
    }

    pub fn install_template_archive(&mut self, path: &Path) -> Result<String, String> {
        let manager = self.template_manager.as_mut().ok_or_else(|| "player-template registry is unavailable".to_owned())?;
        let installed = manager.install_archive(path).map_err(|error| error.to_string())?;
        if installed.engine_version == self.project_engine_version
            && PlayerTemplateTarget::current() == Some(installed.target)
        {
            self.detected_template = Some(installed.binary.clone());
        }
        self.status = format!("Installed player template '{}'", installed.id);
        Ok(self.status.clone())
    }

    pub fn download_template(&mut self, url: &str) -> Result<String, String> {
        let manager = self.template_manager.as_mut().ok_or_else(|| "player-template registry is unavailable".to_owned())?;
        let installed = manager.download_and_install(url).map_err(|error| error.to_string())?;
        if installed.engine_version == self.project_engine_version
            && PlayerTemplateTarget::current() == Some(installed.target)
        {
            self.detected_template = Some(installed.binary.clone());
        }
        self.status = format!("Downloaded player template '{}'", installed.id);
        Ok(self.status.clone())
    }

    pub fn download_compatible_template(
        &mut self,
        catalog_url: &str,
        target: vetrace_build::ExportTarget,
    ) -> Result<String, String> {
        let template_target = PlayerTemplateTarget::for_export_target(target)
            .ok_or_else(|| format!("export target '{}' has no player-template target", target.label()))?;
        let catalog = PlayerTemplateManager::download_catalog(catalog_url)
            .map_err(|error| error.to_string())?;
        let entry = catalog
            .find(template_target, &self.project_engine_version)
            .ok_or_else(|| format!(
                "catalog contains no {:?} player template for engine {}",
                template_target, self.project_engine_version,
            ))?
            .clone();
        let manager = self.template_manager.as_mut()
            .ok_or_else(|| "player-template registry is unavailable".to_owned())?;
        let installed = manager
            .download_catalog_entry(&catalog, &entry.id)
            .map_err(|error| error.to_string())?;
        self.detected_template = Some(installed.binary.clone());
        self.status = format!("Downloaded compatible player template '{}'", installed.id);
        Ok(self.status.clone())
    }

    pub fn remove_template(&mut self, id: &str) -> Result<String, String> {
        let manager = self.template_manager.as_mut().ok_or_else(|| "player-template registry is unavailable".to_owned())?;
        if !manager.remove(id).map_err(|error| error.to_string())? {
            return Err(format!("player template '{id}' is not installed"));
        }
        if self.detected_template.as_ref().is_some_and(|path| !path.exists()) {
            self.detected_template = None;
        }
        self.status = format!("Removed player template '{id}'");
        Ok(self.status.clone())
    }

    pub fn start(
        &mut self,
        project: &VetraceProject,
        preset: ExportPreset,
        template: Option<PathBuf>,
    ) -> Result<String, String> {
        if self.task.is_some() {
            return Err("an export is already running".to_owned());
        }
        preset.validate().map_err(|error| error.to_string())?;
        let player_template = template
            .filter(|path| path.is_file())
            .or_else(|| self.detected_template.clone())
            .ok_or_else(|| {
                "select a prebuilt vetrace-player template before exporting".to_owned()
            })?;
        self.save_preset(project, preset.clone())?;
        let request = BuildRequest {
            project: project.clone(),
            preset: preset.clone(),
            player_template,
            asset_preflight: BuildAssetPreflight::ExistingDatabase,
        };
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = build_project(&request).map_err(|error| error.to_string());
            let _ = sender.send(result);
        });
        self.status = format!("Exporting preset '{}'…", preset.name);
        self.last_report = None;
        self.task = Some(BuildTask { receiver });
        Ok(self.status.clone())
    }

    pub fn update(&mut self) -> Option<Result<BuildReport, String>> {
        let result = match self.task.as_ref()?.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                Err("export worker stopped without returning a result".to_owned())
            }
        };
        self.task = None;
        match &result {
            Ok(report) => {
                self.status = format!(
                    "Export complete: {}",
                    report.output_directory.display()
                );
                self.last_report = Some(report.clone());
            }
            Err(error) => {
                self.status = format!("Export failed: {error}");
            }
        }
        Some(result)
    }

    pub fn open_output(path: &Path) -> Result<(), String> {
        let mut command = if cfg!(target_os = "windows") {
            let mut command = Command::new("explorer");
            command.arg(path);
            command
        } else if cfg!(target_os = "macos") {
            let mut command = Command::new("open");
            command.arg(path);
            command
        } else {
            let mut command = Command::new("xdg-open");
            command.arg(path);
            command
        };
        command.spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open '{}': {error}", path.display()))
    }
}
