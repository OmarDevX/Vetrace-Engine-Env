use super::*;

impl StudioEguiTool {
    pub(super) fn draw_inspector_panel(
        &mut self,
        ctx: &egui::Context,
        snapshot: &StudioSnapshot,
    ) -> egui::InnerResponse<()> {
        egui::SidePanel::right("vetrace_studio_inspector")
            .default_width(340.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();
                let Some(entity) = snapshot.selected else {
                    ui.label("Select an entity in the viewport or scene tree.");
                    return;
                };

                let mut name = snapshot.selected_name.clone();
                ui.horizontal(|ui| {
                    ui.label("Name");
                    if ui.text_edit_singleline(&mut name).changed() {
                        self.command(StudioCommand::Rename {
                            entity,
                            name: name.clone(),
                        });
                    }
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_source("vetrace_studio_inspector_scroll")
                    .show(ui, |ui| {
                        for component in &snapshot.components {
                            self.component_ui(ui, entity, component, snapshot);
                        }
                        self.draw_add_component(ui, entity, snapshot);
                    });
            })
    }

    fn draw_add_component(
        &mut self,
        ui: &mut egui::Ui,
        entity: vetrace_core::Entity,
        snapshot: &StudioSnapshot,
    ) {
        ui.separator();
        ui.label("Add component");
        let selected_text = snapshot
            .addable_components
            .iter()
            .find(|schema| schema.stable_id == self.selected_add_component)
            .map(|schema| schema.display_name.as_str())
            .unwrap_or("Choose component");
        egui::ComboBox::from_id_source("vetrace_studio_add_component")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for schema in &snapshot.addable_components {
                    ui.selectable_value(
                        &mut self.selected_add_component,
                        schema.stable_id.clone(),
                        format!("{} / {}", schema.category, schema.display_name),
                    );
                }
            });
        if ui
            .add_enabled(
                !self.selected_add_component.is_empty(),
                egui::Button::new("Add component"),
            )
            .clicked()
        {
            let component = self.selected_add_component.clone();
            self.command(StudioCommand::AddComponent {
                entity,
                component: component.clone(),
            });
            if component == LUA_SCRIPT_COMPONENT_ID {
                self.script_assets
                    .begin_assignment(entity, &snapshot.selected_name);
                self.bottom_tab = BottomTab::Assets;
            }
            self.selected_add_component.clear();
        }
    }
}
