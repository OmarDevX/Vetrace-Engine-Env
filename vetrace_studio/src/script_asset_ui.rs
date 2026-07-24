use rfd::FileDialog;
use vetrace_asset::AssetKind;
use vetrace_core::{DynamicValue, Entity};
use vetrace_render::egui;

use crate::script_assets::{suggested_script_path, LUA_SCRIPT_COMPONENT_ID};
use crate::protocol::{DraggedAsset, ReflectedFieldSnapshot, StudioCommand, StudioSnapshot};

#[derive(Default)]
pub struct ScriptAssetUi {
    dragged_asset: Option<DraggedAsset>,
    assignment: Option<ScriptAssignmentDialog>,
}

#[derive(Clone, Debug)]
struct ScriptAssignmentDialog {
    entity: Entity,
    project_path: String,
}

impl ScriptAssetUi {
    pub fn dragged_asset_mut(&mut self) -> &mut Option<DraggedAsset> {
        &mut self.dragged_asset
    }

    pub fn begin_assignment(&mut self, entity: Entity, entity_name: &str) {
        self.assignment = Some(ScriptAssignmentDialog {
            entity,
            project_path: suggested_script_path(entity_name),
        });
    }

    pub fn dialog_ui(
        &mut self,
        ctx: &egui::Context,
        snapshot: &StudioSnapshot,
    ) -> Vec<StudioCommand> {
        let mut commands = Vec::new();
        let Some(mut dialog) = self.assignment.take() else { return commands; };
        let mut keep_open = true;
        egui::Window::new("Assign Lua Script")
            .id(egui::Id::new("vetrace_studio_assign_lua_script"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Create a new script or assign an existing project script.");
                ui.add_space(4.0);
                ui.label("New script path");
                ui.add(
                    egui::TextEdit::singleline(&mut dialog.project_path)
                        .desired_width(420.0)
                        .hint_text("assets/scripts/player.lua"),
                );
                ui.label(
                    egui::RichText::new(
                        "Scripts must be inside assets/scripts/ and use the .lua extension.",
                    )
                    .small(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Create & Assign").clicked() {
                        commands.push(StudioCommand::CreateLuaScript {
                            entity: dialog.entity,
                            project_path: dialog.project_path.clone(),
                        });
                        keep_open = false;
                    }
                    if ui.button("Browse Existing…").clicked() {
                        if let Some(source) = choose_existing_script(snapshot) {
                            commands.push(StudioCommand::AssignLuaScript {
                                entity: dialog.entity,
                                source,
                            });
                            keep_open = false;
                        }
                    }
                    if ui.button("Leave Empty").clicked() {
                        keep_open = false;
                    }
                });
                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "You can also type a project path in the Inspector or drag a Lua asset from Assets onto the Script field.",
                    )
                    .small(),
                );
            });
        if keep_open {
            self.assignment = Some(dialog);
        }
        commands
    }

    pub fn field_ui(
        &mut self,
        ui: &mut egui::Ui,
        entity: Entity,
        field: &ReflectedFieldSnapshot,
        snapshot: &StudioSnapshot,
    ) -> Vec<StudioCommand> {
        let mut commands = Vec::new();
        let current = match &field.value {
            DynamicValue::String(value) => value.clone(),
            _ => String::new(),
        };
        let mut edited = current.clone();
        let row = ui.group(|ui| {
            ui.label(egui::RichText::new("Script").strong());
            let available_width = ui.available_width().max(120.0);
            if ui
                .add(
                    egui::TextEdit::singleline(&mut edited)
                        .desired_width(available_width)
                        .hint_text("assets/scripts/player.lua"),
                )
                .changed()
            {
                commands.push(StudioCommand::SetField {
                    entity,
                    component: LUA_SCRIPT_COMPONENT_ID.to_string(),
                    path: field.path.clone(),
                    value: DynamicValue::String(edited.clone()),
                });
            }

            ui.horizontal_wrapped(|ui| {
                if ui.button("New…").on_hover_text("Create and assign a new Lua script").clicked() {
                    self.begin_assignment(entity, &snapshot.selected_name);
                }
                if ui.button("Browse…").on_hover_text("Choose an existing project Lua script").clicked() {
                    if let Some(source) = choose_existing_script(snapshot) {
                        commands.push(StudioCommand::AssignLuaScript { entity, source });
                    }
                }
                if ui
                    .add_enabled(!current.is_empty(), egui::Button::new("Open"))
                    .on_hover_text("Open in the built-in Script Editor")
                    .clicked()
                {
                    commands.push(StudioCommand::OpenScript(current.clone().into()));
                }
                if ui
                    .add_enabled(!current.is_empty(), egui::Button::new("Clear"))
                    .clicked()
                {
                    commands.push(StudioCommand::SetField {
                        entity,
                        component: LUA_SCRIPT_COMPONENT_ID.to_string(),
                        path: field.path.clone(),
                        value: DynamicValue::String(String::new()),
                    });
                }
            });

            let dragged_is_script = self
                .dragged_asset
                .as_ref()
                .map(|asset| asset.kind == AssetKind::Script);
            let hint = match dragged_is_script {
                Some(true) => "Release to assign this Lua script",
                Some(false) => "Only Lua script assets can be assigned here",
                None => "Drag a Lua script here from Assets",
            };
            ui.label(egui::RichText::new(hint).small().italics());
            if let Some(description) = &field.schema.description {
                ui.label(egui::RichText::new(description).small());
            }
        });

        let can_drop_script = self
            .dragged_asset
            .as_ref()
            .is_some_and(|asset| asset.kind == AssetKind::Script);
        if row.response.hovered()
            && ui.input(|input| input.pointer.any_released())
            && can_drop_script
        {
            let asset = self.dragged_asset.take().expect("Lua script drag checked");
            commands.push(StudioCommand::AssignLuaScript {
                entity,
                source: asset.path,
            });
        }
        commands
    }

    pub fn finish_frame(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.pointer.any_released()) {
            self.dragged_asset = None;
        }
    }
}

fn choose_existing_script(snapshot: &StudioSnapshot) -> Option<std::path::PathBuf> {
    FileDialog::new()
        .set_title("Choose Lua Script")
        .set_directory(snapshot.project_root.join("assets/scripts"))
        .add_filter("Lua script", &["lua"])
        .pick_file()
}
