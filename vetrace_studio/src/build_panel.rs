use std::path::PathBuf;

use rfd::FileDialog;
use vetrace_build::{
    CompressionMode, ExportPreset, ExportTarget,
};
use vetrace_project::ProjectPath;
use vetrace_render::egui;

use crate::protocol::{BuildSnapshot, StudioCommand};

#[derive(Default)]
pub struct BuildPanelState {
    selected_preset: String,
    draft: Option<BuildPresetDraft>,
    player_template: String,
    template_url: String,
    template_catalog_url: String,
}

#[derive(Clone)]
struct BuildPresetDraft {
    original_name: String,
    is_new: bool,
    name: String,
    target: ExportTarget,
    output_directory: String,
    executable_name: String,
    package_name: String,
    compression: CompressionMode,
    include_asset_database: bool,
}

impl BuildPresetDraft {
    fn from_preset(preset: &ExportPreset) -> Self {
        Self {
            original_name: preset.name.clone(),
            is_new: false,
            name: preset.name.clone(),
            target: preset.target,
            output_directory: preset.output_directory.to_string(),
            executable_name: preset.executable_name.clone(),
            package_name: preset.package_name.clone(),
            compression: preset.compression,
            include_asset_database: preset.include_asset_database,
        }
    }

    fn to_preset(&self) -> Result<ExportPreset, String> {
        let preset = ExportPreset {
            name: self.name.trim().to_owned(),
            target: self.target,
            output_directory: ProjectPath::new(self.output_directory.trim())
                .map_err(|error| error.to_string())?,
            executable_name: self.executable_name.trim().to_owned(),
            package_name: self.package_name.trim().to_owned(),
            compression: self.compression,
            include_asset_database: self.include_asset_database,
        };
        preset.validate().map_err(|error| error.to_string())?;
        Ok(preset)
    }
}

impl BuildPanelState {
    pub fn ui(&mut self, ui: &mut egui::Ui, snapshot: &BuildSnapshot) -> Vec<StudioCommand> {
        self.synchronize(snapshot);
        let mut commands = Vec::new();
        draw_build_header(ui, snapshot);
        self.draw_preset_selector(ui, snapshot);

        let Some(draft) = self.draft.as_mut() else {
            ui.label("No export preset is available.");
            return commands;
        };

        draw_preset_fields(ui, draft);
        draw_player_template_selector(ui, &mut self.player_template, snapshot);
        draw_template_manager(
            ui,
            &mut self.player_template,
            &mut self.template_url,
            &mut self.template_catalog_url,
            draft.target,
            snapshot,
            &mut commands,
        );
        draw_build_actions(
            ui,
            draft,
            &mut self.selected_preset,
            &self.player_template,
            snapshot,
            &mut commands,
        );
        draw_last_report(ui, snapshot);
        commands
    }

    fn draw_preset_selector(&mut self, ui: &mut egui::Ui, snapshot: &BuildSnapshot) {
        ui.horizontal(|ui| {
            ui.label("Preset");
            egui::ComboBox::from_id_source("vetrace_export_preset")
                .selected_text(if self.selected_preset.is_empty() {
                    "Choose preset"
                } else {
                    self.selected_preset.as_str()
                })
                .show_ui(ui, |ui| {
                    for preset in &snapshot.presets {
                        if ui
                            .selectable_value(
                                &mut self.selected_preset,
                                preset.name.clone(),
                                &preset.name,
                            )
                            .changed()
                        {
                            self.draft = Some(BuildPresetDraft::from_preset(preset));
                        }
                    }
                });
            if ui.button("New preset").clicked() {
                let mut preset = ExportPreset::default();
                preset.name = "New Preset".to_owned();
                self.selected_preset = preset.name.clone();
                let mut draft = BuildPresetDraft::from_preset(&preset);
                draft.is_new = true;
                draft.original_name.clear();
                self.draft = Some(draft);
            }
        });
    }

    fn synchronize(&mut self, snapshot: &BuildSnapshot) {
        if self.player_template.is_empty() {
            if let Some(path) = &snapshot.detected_player_template {
                self.player_template = path.display().to_string();
            }
        }
        let selected_exists = snapshot.presets.iter()
            .any(|preset| preset.name == self.selected_preset);
        if let Some(draft) = self.draft.as_mut() {
            if draft.is_new && selected_exists {
                draft.is_new = false;
                draft.original_name = self.selected_preset.clone();
            } else if draft.is_new {
                return;
            }
        }
        if self.selected_preset.is_empty() || !selected_exists {
            self.selected_preset = snapshot.default_preset.clone();
            self.draft = snapshot.presets.iter()
                .find(|preset| preset.name == self.selected_preset)
                .or_else(|| snapshot.presets.first())
                .map(BuildPresetDraft::from_preset);
        } else if self.draft.as_ref().is_some_and(|draft| {
            draft.original_name != self.selected_preset
        }) {
            self.draft = snapshot.presets.iter()
                .find(|preset| preset.name == self.selected_preset)
                .map(BuildPresetDraft::from_preset);
        }
    }
}

fn draw_build_header(ui: &mut egui::Ui, snapshot: &BuildSnapshot) {
    ui.horizontal(|ui| {
        ui.heading("Build & Export");
        if snapshot.running {
            ui.spinner();
        }
        ui.label(&snapshot.status);
    });
    ui.separator();
}

fn draw_preset_fields(ui: &mut egui::Ui, draft: &mut BuildPresetDraft) {
    egui::Grid::new("vetrace_build_fields")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Name");
            ui.text_edit_singleline(&mut draft.name);
            ui.end_row();

            ui.label("Target");
            egui::ComboBox::from_id_source("vetrace_export_target")
                .selected_text(draft.target.label())
                .show_ui(ui, |ui| {
                    for target in ExportTarget::ALL {
                        ui.selectable_value(&mut draft.target, target, target.label());
                    }
                });
            ui.end_row();

            ui.label("Output");
            ui.text_edit_singleline(&mut draft.output_directory);
            ui.end_row();
            ui.label("Executable");
            ui.text_edit_singleline(&mut draft.executable_name);
            ui.end_row();
            ui.label("Package");
            ui.text_edit_singleline(&mut draft.package_name);
            ui.end_row();

            ui.label("Compression");
            egui::ComboBox::from_id_source("vetrace_export_compression")
                .selected_text(draft.compression.label())
                .show_ui(ui, |ui| {
                    for compression in CompressionMode::ALL {
                        ui.selectable_value(
                            &mut draft.compression,
                            compression,
                            compression.label(),
                        );
                    }
                });
            ui.end_row();

            ui.label("Asset database");
            ui.checkbox(&mut draft.include_asset_database, "Include stable IDs");
            ui.end_row();
        });
}

fn draw_player_template_selector(
    ui: &mut egui::Ui,
    player_template: &mut String,
    snapshot: &BuildSnapshot,
) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Player template");
        let width = (ui.available_width() - 130.0).max(120.0);
        ui.add_sized(
            [width, ui.spacing().interact_size.y],
            egui::TextEdit::singleline(player_template).hint_text("prebuilt vetrace-player"),
        );
        if ui.button("Browse…").clicked() {
            let mut dialog = FileDialog::new().set_title("Select prebuilt vetrace-player");
            let current_template = PathBuf::from(player_template.as_str());
            if let Some(parent) = current_template.parent().filter(|path| path.is_dir()) {
                dialog = dialog.set_directory(parent);
            }
            if let Some(path) = dialog.pick_file() {
                *player_template = path.display().to_string();
            }
        }
    });
    if let Some(detected) = &snapshot.detected_player_template {
        ui.horizontal(|ui| {
            ui.label(format!("Detected: {}", detected.display()));
            if ui.small_button("Use detected").clicked() {
                *player_template = detected.display().to_string();
            }
        });
    } else {
        ui.label(
            egui::RichText::new(
                "Build vetrace-player once, place it beside Studio, set VETRACE_PLAYER_TEMPLATE, or browse to a prebuilt binary.",
            )
            .small()
            .weak(),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_template_manager(
    ui: &mut egui::Ui,
    player_template: &mut String,
    template_url: &mut String,
    template_catalog_url: &mut String,
    target: ExportTarget,
    snapshot: &BuildSnapshot,
    commands: &mut Vec<StudioCommand>,
) {
    ui.separator();
    egui::CollapsingHeader::new("Player Template Manager")
        .default_open(false)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Install binary…").clicked() {
                    if let Some(path) = FileDialog::new()
                        .set_title("Install vetrace-player binary")
                        .pick_file()
                    {
                        commands.push(StudioCommand::InstallPlayerTemplate(path));
                    }
                }
                if ui.button("Install bundle…").clicked() {
                    if let Some(path) = FileDialog::new()
                        .set_title("Install Vetrace player-template bundle")
                        .add_filter("Vetrace Template", &["vtemplate", "zip"])
                        .pick_file()
                    {
                        commands.push(StudioCommand::InstallPlayerTemplateArchive(path));
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Bundle URL");
                ui.add(egui::TextEdit::singleline(template_url).desired_width(420.0));
                let valid_url = template_url.starts_with("https://")
                    || template_url.starts_with("http://");
                if ui
                    .add_enabled(valid_url, egui::Button::new("Download bundle"))
                    .clicked()
                {
                    commands.push(StudioCommand::DownloadPlayerTemplate(
                        template_url.trim().to_owned(),
                    ));
                }
            });
            ui.horizontal(|ui| {
                ui.label("Catalog URL");
                ui.add(
                    egui::TextEdit::singleline(template_catalog_url).desired_width(420.0),
                );
                let valid_url = template_catalog_url.starts_with("https://")
                    || template_catalog_url.starts_with("http://");
                if ui
                    .add_enabled(valid_url, egui::Button::new("Install compatible"))
                    .clicked()
                {
                    commands.push(StudioCommand::DownloadCompatiblePlayerTemplate {
                        catalog_url: template_catalog_url.trim().to_owned(),
                        target,
                    });
                }
            });
            if snapshot.installed_templates.is_empty() {
                ui.label(
                    egui::RichText::new("No managed templates installed.")
                        .small()
                        .weak(),
                );
            }
            for template in &snapshot.installed_templates {
                ui.horizontal(|ui| {
                    ui.monospace(&template.id);
                    ui.label(format!("{} · {}", template.engine_version, template.target));
                    if ui.small_button("Use").clicked() {
                        *player_template = template.binary.display().to_string();
                    }
                    if ui.small_button("Remove").clicked() {
                        commands.push(StudioCommand::RemovePlayerTemplate(template.id.clone()));
                    }
                });
            }
        });
}

fn draw_build_actions(
    ui: &mut egui::Ui,
    draft: &BuildPresetDraft,
    selected_preset: &mut String,
    player_template: &str,
    snapshot: &BuildSnapshot,
    commands: &mut Vec<StudioCommand>,
) {
    let preset_result = draft.to_preset();
    if let Err(error) = &preset_result {
        ui.label(egui::RichText::new(error).small().strong());
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(preset_result.is_ok(), egui::Button::new("Save preset"))
            .clicked()
        {
            if let Ok(preset) = preset_result.clone() {
                *selected_preset = preset.name.clone();
                commands.push(StudioCommand::SaveExportPreset(preset));
            }
        }
        let template = PathBuf::from(player_template.trim());
        let can_export = preset_result.is_ok() && template.is_file() && !snapshot.running;
        if ui
            .add_enabled(can_export, egui::Button::new("Export project"))
            .clicked()
        {
            if let Ok(preset) = preset_result.clone() {
                commands.push(StudioCommand::BuildProject {
                    preset,
                    player_template: template,
                });
            }
        }
        if let Some(report) = &snapshot.last_report {
            if ui.button("Open output folder").clicked() {
                commands.push(StudioCommand::OpenBuildFolder(
                    report.output_directory.clone(),
                ));
            }
        }
    });
}

fn draw_last_report(ui: &mut egui::Ui, snapshot: &BuildSnapshot) {
    let Some(report) = &snapshot.last_report else {
        return;
    };
    ui.separator();
    ui.strong("Last export");
    egui::Grid::new("vetrace_last_build").show(ui, |ui| {
        ui.label("Output");
        ui.monospace(report.output_directory.display().to_string());
        ui.end_row();
        ui.label("Executable");
        ui.monospace(report.executable.display().to_string());
        ui.end_row();
        ui.label("Package");
        ui.monospace(report.package.display().to_string());
        ui.end_row();
        ui.label("Entries");
        ui.label(report.package_entries.to_string());
        ui.end_row();
        ui.label("Package size");
        ui.label(format_bytes(report.package_bytes));
        ui.end_row();
        ui.label("BLAKE3");
        ui.monospace(&report.package_blake3);
        ui.end_row();
    });
    for warning in &report.warnings {
        ui.label(egui::RichText::new(format!("Warning: {warning}")).small());
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB { format!("{:.1} GiB", bytes / GIB) }
    else if bytes >= MIB { format!("{:.1} MiB", bytes / MIB) }
    else if bytes >= KIB { format!("{:.1} KiB", bytes / KIB) }
    else { format!("{} B", bytes as u64) }
}
