use super::*;

impl ScriptEditorPanel {
    pub(super) fn sync_active_view(&mut self, state: &mut StudioScriptState, active_index: usize) {
        let path = state.workspace.documents()[active_index].path.clone();
        if self.active_path.as_ref() == Some(&path) { return; }
        self.active_path = Some(path.clone());
        self.references.clear();
        self.rename_symbol.clear();
        self.rename_file_path = project_relative_script(&path);
        let view = state.view_states.get(&path).cloned().unwrap_or(ScriptViewState {
            cursor_byte: 0,
            line: 1,
        });
        self.cursor_byte = view.cursor_byte.min(state.workspace.documents()[active_index].text.len());
        self.active_line = view.line.max(1);
        self.pending_cursor_byte = Some(self.cursor_byte);
    }

    pub(super) fn external_change_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &StudioScriptState,
        active_index: usize,
        commands: &mut Vec<StudioCommand>,
    ) {
        let path = &state.workspace.documents()[active_index].path;
        let Some(change) = state.external_changes.iter().find(|change| &change.path == path) else { return; };
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let message = match change.kind {
                    ExternalChangeKind::Modified if change.has_local_changes => "This script changed on disk and in Studio.",
                    ExternalChangeKind::Modified => "This script changed on disk.",
                    ExternalChangeKind::Deleted => "This script was deleted outside Studio.",
                };
                ui.label(egui::RichText::new(message).color(egui::Color32::YELLOW));
                if change.kind == ExternalChangeKind::Modified
                    && ui.button("Reload disk").clicked()
                {
                    commands.push(StudioCommand::ResolveScriptExternal {
                        path: path.clone(),
                        resolution: ExternalChangeResolution::Reload,
                    });
                }
                if ui.button("Keep Studio").clicked() {
                    commands.push(StudioCommand::ResolveScriptExternal {
                        path: path.clone(),
                        resolution: ExternalChangeResolution::KeepLocal,
                    });
                }
                if change.kind == ExternalChangeKind::Modified
                    && change.has_local_changes
                    && ui.button("Merge").clicked()
                {
                    commands.push(StudioCommand::ResolveScriptExternal {
                        path: path.clone(),
                        resolution: ExternalChangeResolution::Merge,
                    });
                }
            });
        });
    }

}
