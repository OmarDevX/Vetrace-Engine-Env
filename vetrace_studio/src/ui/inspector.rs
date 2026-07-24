use super::*;

impl StudioEguiTool {
    pub(super) fn component_ui(
        &mut self,
        ui: &mut egui::Ui,
        entity: vetrace_core::Entity,
        component: &ReflectedComponentSnapshot,
        snapshot: &StudioSnapshot,
    ) {
        let title = format!("{}  ·  {}", component.schema.display_name, component.schema.category);
        egui::CollapsingHeader::new(title)
            .id_source(("component", &component.schema.stable_id))
            .default_open(
                component.schema.stable_id == "vetrace.core.transform"
                    || component.schema.stable_id == LUA_SCRIPT_COMPONENT_ID,
            )
            .show(ui, |ui| {
                if let Some(description) = &component.schema.description {
                    ui.label(egui::RichText::new(description).small());
                }
                for field in &component.fields {
                    self.field_ui(ui, entity, &component.schema.stable_id, field, snapshot);
                }
                if component.schema.removable {
                    if ui.small_button("Remove component").clicked() {
                        self.command(StudioCommand::RemoveComponent {
                            entity,
                            component: component.schema.stable_id.clone(),
                        });
                    }
                }
            });
    }

    fn field_ui(
        &mut self,
        ui: &mut egui::Ui,
        entity: vetrace_core::Entity,
        component: &str,
        field: &ReflectedFieldSnapshot,
        snapshot: &StudioSnapshot,
    ) {
        if is_lua_script_field(component, &field.path.to_string()) {
            for command in self.script_assets.field_ui(ui, entity, field, snapshot) {
                self.command(command);
            }
            return;
        }

        let mut value = field.value.clone();
        if draw_dynamic_value(ui, component, &field.schema, &field.path, &mut value) {
            self.command(StudioCommand::SetField {
                entity,
                component: component.to_string(),
                path: field.path.clone(),
                value,
            });
        }
    }
}
