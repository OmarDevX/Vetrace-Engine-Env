use super::*;

impl ScriptEditorPanel {
    pub(super) fn go_to_definition(&mut self, state: &StudioScriptState, active_index: usize) {
        let document = &state.workspace.documents()[active_index];
        let Some(service) = state.workspace.registry().get(&document.language_id) else { return; };
        if let Some(range) = service.definition(&document.text, self.cursor_byte) {
            self.pending_cursor_byte = Some(range.start);
            self.target_line = Some(line_for_offset(&document.text, range.start));
            self.status = Some("Jumped to definition".into());
        } else {
            self.status = Some("No definition found for the symbol under the cursor".into());
        }
    }

    pub(super) fn find_references(&mut self, state: &StudioScriptState, active_index: usize) {
        let document = &state.workspace.documents()[active_index];
        let Some(service) = state.workspace.registry().get(&document.language_id) else { return; };
        self.references = service.references(&document.text, self.cursor_byte);
        self.status = Some(format!("Found {} reference(s)", self.references.len()));
    }

    pub(super) fn rename_symbol_ui(&mut self, ui: &mut egui::Ui, state: &mut StudioScriptState, active_index: usize) {
        ui.horizontal(|ui| {
            ui.label("New symbol name");
            let response = ui.add(egui::TextEdit::singleline(&mut self.rename_symbol).desired_width(180.0));
            let apply = ui.button("Rename all references").clicked()
                || (response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if apply {
                let language_id = state.workspace.documents()[active_index].language_id.clone();
                if let Some(service) = state.workspace.registry().get(&language_id) {
                    let source = state.workspace.documents()[active_index].text.clone();
                    match service.rename_edits(&source, self.cursor_byte, self.rename_symbol.trim()) {
                        Ok(edits) => {
                            let count = edits.len();
                            if state.workspace.documents_mut()[active_index].apply_edits(&edits).is_ok() {
                                let _ = state.workspace.analyze(active_index);
                                self.status = Some(format!("Renamed {count} reference(s)"));
                                self.show_rename_symbol = false;
                                self.last_edit = Some(Instant::now());
                            }
                        }
                        Err(error) => self.status = Some(error.to_string()),
                    }
                }
            }
            if ui.button("Cancel").clicked() { self.show_rename_symbol = false; }
        });
    }

}
