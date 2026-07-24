use super::*;

impl ScriptEditorPanel {
    pub(super) fn tabs_ui(&mut self, ui: &mut egui::Ui, state: &mut StudioScriptState) {
        let tabs = state.workspace.documents().iter().enumerate().map(|(index, document)| {
            let name = document.path.file_name().and_then(|name| name.to_str()).unwrap_or("script.lua");
            (index, name.to_owned(), document.is_dirty())
        }).collect::<Vec<_>>();
        ui.horizontal_wrapped(|ui| {
            for (index, name, dirty) in tabs {
                let selected = state.workspace.active_index() == Some(index);
                let label = if dirty { format!("{name} •") } else { name };
                if ui.selectable_label(selected, label).clicked() {
                    state.workspace.set_active(index);
                    self.completion_open = false;
                }
                if ui.small_button("×").on_hover_text("Close script").clicked() {
                    if dirty {
                        self.pending_close = Some(index);
                    } else {
                        let _ = state.workspace.close(index, true);
                    }
                }
            }
            if state.workspace.documents().is_empty() {
                ui.label("No scripts open");
            }
        });
    }

}
