use std::collections::HashMap;

use rfd::FileDialog;
use vetrace_asset::{AssetDiagnosticSeverity, AssetId, AssetKind, AssetStatus};
use vetrace_render::egui;

use crate::protocol::{AssetRow, DraggedAsset, StudioCommand, StudioSnapshot};

#[derive(Default)]
pub struct AssetBrowserState {
    filter: String,
    kind_filter: Option<AssetKind>,
    status_filter: Option<AssetStatus>,
    selected: Option<AssetId>,
    thumbnails: HashMap<AssetId, (String, egui::TextureHandle)>,
}

impl AssetBrowserState {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &StudioSnapshot,
        dragged_asset: &mut Option<DraggedAsset>,
    ) -> Vec<StudioCommand> {
        let mut commands = Vec::new();
        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(180.0));
            egui::ComboBox::from_id_source("vetrace_asset_kind_filter")
                .selected_text(self.kind_filter.as_ref().map(AssetKind::label).unwrap_or("All types"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.kind_filter, None, "All types");
                    for kind in [
                        AssetKind::Scene, AssetKind::Script, AssetKind::Model, AssetKind::Texture,
                        AssetKind::Audio, AssetKind::Font, AssetKind::Shader, AssetKind::Material,
                        AssetKind::Data, AssetKind::Unknown,
                    ] {
                        let label = kind.label().to_string();
                        ui.selectable_value(&mut self.kind_filter, Some(kind), label);
                    }
                });
            egui::ComboBox::from_id_source("vetrace_asset_status_filter")
                .selected_text(self.status_filter.map(asset_status_label).unwrap_or("All states"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.status_filter, None, "All states");
                    for status in [
                        AssetStatus::Ready, AssetStatus::NeedsImport, AssetStatus::Missing,
                        AssetStatus::Failed, AssetStatus::Unsupported,
                    ] {
                        ui.selectable_value(&mut self.status_filter, Some(status), asset_status_label(status));
                    }
                });
            if ui.button("Import Files…").clicked() {
                if let Some(files) = FileDialog::new()
                    .set_title("Import Assets")
                    .set_directory(snapshot.project_root.join("assets"))
                    .pick_files()
                {
                    commands.push(StudioCommand::ImportAssetFiles(files));
                }
            }
            if ui.button("Refresh").clicked() { commands.push(StudioCommand::RefreshAssets); }
            ui.menu_button("Cache", |ui| {
                if ui.button("Reimport all").clicked() {
                    commands.push(StudioCommand::ReimportAllAssets);
                    ui.close_menu();
                }
                if ui.button("Prune orphan cache").clicked() {
                    commands.push(StudioCommand::PruneAssetCache);
                    ui.close_menu();
                }
                if ui.button("Clear and rebuild cache").clicked() {
                    commands.push(StudioCommand::ClearAssetCache);
                    ui.close_menu();
                }
            });
            ui.separator();
            ui.label("Drag assets from the list onto compatible Inspector fields");
            ui.separator();
            ui.label(format!(
                "{} assets · {} imported files · {} · {} orphan caches",
                snapshot.assets.len(), snapshot.asset_cache.imported_files,
                format_bytes(snapshot.asset_cache.bytes), snapshot.asset_cache.orphan_directories,
            ));
        });

        let filter = self.filter.to_ascii_lowercase();
        let filtered: Vec<&AssetRow> = snapshot.assets.iter().filter(|asset| {
            (filter.is_empty() || asset.path.to_ascii_lowercase().contains(&filter))
                && self.kind_filter.as_ref().map_or(true, |kind| kind == &asset.kind)
                && self.status_filter.map_or(true, |status| status == asset.status)
        }).collect();

        ui.separator();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_min_width((ui.available_width() * 0.58).max(360.0));
                egui::ScrollArea::vertical().id_source("vetrace_asset_list").show(ui, |ui| {
                    egui::Grid::new("vetrace_asset_grid")
                        .striped(true)
                        .min_col_width(80.0)
                        .show(ui, |ui| {
                            ui.strong("Asset"); ui.strong("Type"); ui.strong("State"); ui.strong("Size"); ui.end_row();
                            for asset in filtered {
                                let selected = self.selected == Some(asset.id);
                                let label = if selected {
                                    egui::RichText::new(&asset.path).strong()
                                } else {
                                    egui::RichText::new(&asset.path)
                                };
                                let response = ui.add(
                                    egui::Label::new(label).sense(egui::Sense::click_and_drag()),
                                );
                                if response.clicked() {
                                    self.selected = Some(asset.id);
                                }
                                if response.double_clicked() && asset.kind == AssetKind::Script {
                                    commands.push(StudioCommand::OpenScript(asset.path.clone().into()));
                                }
                                if response.drag_started() {
                                    self.selected = Some(asset.id);
                                    *dragged_asset = Some(DraggedAsset {
                                        path: asset.path.clone().into(),
                                        file_name: asset.file_name.clone(),
                                        kind: asset.kind.clone(),
                                    });
                                }
                                if response.dragged() {
                                    response.on_hover_cursor(egui::CursorIcon::Grabbing);
                                }
                                ui.label(asset.kind.label());
                                ui.label(asset_status_label(asset.status));
                                ui.label(format_bytes(asset.size));
                                ui.end_row();
                            }
                        });
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                ui.set_min_width(300.0);
                if let Some(asset) = self.selected.and_then(|id| snapshot.assets.iter().find(|asset| asset.id == id)) {
                    ui.horizontal(|ui| {
                        ui.heading(&asset.file_name);
                        if asset.kind == AssetKind::Script && ui.button("Open Script").clicked() {
                            commands.push(StudioCommand::OpenScript(asset.path.clone().into()));
                        }
                        if ui.button("Reimport").clicked() { commands.push(StudioCommand::ReimportAsset(asset.id)); }
                    });
                    ui.monospace(&asset.path);
                    if let Some(thumbnail) = self.thumbnail(ui.ctx(), asset) {
                        ui.add_space(4.0);
                        ui.image((thumbnail.id(), egui::vec2(192.0, 192.0)));
                    }
                    egui::Grid::new("vetrace_asset_details").show(ui, |ui| {
                        ui.strong("ID"); ui.monospace(asset.id.to_string()); ui.end_row();
                        ui.strong("Type"); ui.label(asset.kind.label()); ui.end_row();
                        ui.strong("State"); ui.label(asset_status_label(asset.status)); ui.end_row();
                        ui.strong("Importer"); ui.label(asset.importer.as_deref().unwrap_or("None")); ui.end_row();
                        ui.strong("Hash"); ui.monospace(asset.hash.chars().take(16).collect::<String>()); ui.end_row();
                    });
                    if let Some(error) = &asset.error { ui.label(egui::RichText::new(error).strong()); }
                    if !asset.dependencies.is_empty() {
                        ui.separator(); ui.strong("Dependencies");
                        for (dependency, missing) in &asset.dependencies {
                            ui.label(if *missing { format!("Missing: {dependency}") } else { dependency.clone() });
                        }
                    }
                    if !asset.outputs.is_empty() {
                        ui.separator(); ui.strong("Imported outputs");
                        for output in &asset.outputs { ui.monospace(output); }
                    }
                } else {
                    ui.label("Select an asset to inspect its stable ID, importer, dependencies, and outputs.");
                }
            });
        });

        if !snapshot.asset_diagnostics.is_empty() {
            ui.separator();
            ui.collapsing(format!("Diagnostics ({})", snapshot.asset_diagnostics.len()), |ui| {
                for diagnostic in &snapshot.asset_diagnostics {
                    let severity = match diagnostic.severity {
                        AssetDiagnosticSeverity::Info => "Info",
                        AssetDiagnosticSeverity::Warning => "Warning",
                        AssetDiagnosticSeverity::Error => "Error",
                    };
                    ui.label(format!("[{severity}] {}: {}", diagnostic.code, diagnostic.message));
                }
            });
        }
        commands
    }

    fn thumbnail(&mut self, ctx: &egui::Context, asset: &AssetRow) -> Option<egui::TextureHandle> {
        if let Some((hash, texture)) = self.thumbnails.get(&asset.id) {
            if hash == &asset.hash { return Some(texture.clone()); }
        }
        let path = asset.thumbnail.as_ref()?;
        let decoded = image::open(path).ok()?.to_rgba8();
        let size = [decoded.width() as usize, decoded.height() as usize];
        let pixels = decoded.into_raw();
        let image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        let texture = ctx.load_texture(
            format!("asset-thumbnail-{}", asset.id),
            image,
            egui::TextureOptions::LINEAR,
        );
        self.thumbnails.insert(asset.id, (asset.hash.clone(), texture.clone()));
        Some(texture)
    }
}

fn asset_status_label(status: AssetStatus) -> &'static str {
    match status {
        AssetStatus::Ready => "Ready",
        AssetStatus::NeedsImport => "Needs import",
        AssetStatus::Missing => "Missing",
        AssetStatus::Failed => "Failed",
        AssetStatus::Unsupported => "Unsupported",
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
