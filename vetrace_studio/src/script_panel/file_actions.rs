use super::*;

impl ScriptEditorPanel {
    pub(super) fn file_actions_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &StudioScriptState,
        active_index: usize,
        commands: &mut Vec<StudioCommand>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Script path");
            ui.add(egui::TextEdit::singleline(&mut self.rename_file_path).desired_width(340.0));
            if ui.button("Rename / Move").clicked() {
                commands.push(StudioCommand::RenameScript {
                    index: active_index,
                    project_path: self.rename_file_path.trim().to_owned(),
                });
                self.show_file_actions = false;
            }
            if ui.button("Delete…").clicked() {
                self.pending_delete = Some(active_index);
                self.show_file_actions = false;
            }
            if ui.button("Close").clicked() { self.show_file_actions = false; }
            ui.weak("References in project manifests, Lua files, scenes, prefabs, and materials are updated when renamed.");
            let _ = state;
        });
    }

    pub(super) fn outline_ui(&mut self, ui: &mut egui::Ui, state: &StudioScriptState, active_index: usize) {
        let document = &state.workspace.documents()[active_index];
        let Some(service) = state.workspace.registry().get(&document.language_id) else { return; };
        let symbols = service.symbols(&document.text);
        egui::CollapsingHeader::new(format!("Symbols ({})", symbols.len()))
            .default_open(true)
            .show(ui, |ui| {
                for symbol in symbols {
                    let icon = match symbol.kind {
                        SymbolKind::Function => "ƒ",
                        SymbolKind::Local => "L",
                        SymbolKind::Parameter => "P",
                        SymbolKind::Property => "◇",
                        SymbolKind::Module => "M",
                    };
                    let line = line_for_offset(&document.text, symbol.selection_range.start);
                    if ui.selectable_label(false, format!("{icon} {}  :{line}", symbol.name)).clicked() {
                        self.pending_cursor_byte = Some(symbol.selection_range.start);
                        self.target_line = Some(line);
                    }
                }
            });
    }

    pub(super) fn references_ui(&mut self, ui: &mut egui::Ui, state: &StudioScriptState, active_index: usize) {
        let document = &state.workspace.documents()[active_index];
        egui::CollapsingHeader::new(format!("References ({})", self.references.len()))
            .default_open(true)
            .show(ui, |ui| {
                for range in self.references.clone() {
                    let line = line_for_offset(&document.text, range.start);
                    let preview_range = line_byte_range(&document.text, line);
                    let preview = document.text[preview_range.start..preview_range.end].trim();
                    if ui.selectable_label(false, format!("{line}: {preview}")).clicked() {
                        self.pending_cursor_byte = Some(range.start);
                        self.target_line = Some(line);
                    }
                }
                if ui.button("Clear references").clicked() { self.references.clear(); }
            });
    }

    pub(super) fn delete_confirmation_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &StudioScriptState,
        index: usize,
        commands: &mut Vec<StudioCommand>,
    ) {
        let Some(document) = state.workspace.documents().get(index) else {
            self.pending_delete = None;
            return;
        };
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(format!("Permanently delete {}?", document.path.display())).color(egui::Color32::LIGHT_RED));
            if ui.button("Delete").clicked() {
                commands.push(StudioCommand::DeleteScript { index, discard: true });
                self.pending_delete = None;
            }
            if ui.button("Cancel").clicked() { self.pending_delete = None; }
        });
    }

}
