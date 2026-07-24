use super::*;

impl ScriptEditorPanel {
    pub(super) fn close_confirmation_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut StudioScriptState,
        index: usize,
        commands: &mut Vec<StudioCommand>,
    ) {
        let Some(document) = state.workspace.documents().get(index) else {
            self.pending_close = None;
            return;
        };
        let path = document.path.clone();
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Save changes to {}?", path.display()));
            if ui.button("Save and close").clicked() {
                commands.push(StudioCommand::SaveAndCloseScript(index));
                self.pending_close = None;
            }
            if ui.button("Discard").clicked() {
                let _ = state.workspace.close(index, true);
                self.pending_close = None;
            }
            if ui.button("Cancel").clicked() { self.pending_close = None; }
        });
    }
}
