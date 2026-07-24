use super::*;

impl ScriptEditorPanel {
    pub(super) fn search_ui(&mut self, ui: &mut egui::Ui, state: &mut StudioScriptState, active_index: usize) {
        ui.horizontal(|ui| {
            ui.label("Find");
            let find_response = ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(180.0));
            ui.label("Replace");
            ui.add(egui::TextEdit::singleline(&mut self.replace).desired_width(180.0));
            if !self.search.is_empty()
                && (ui.button("Next").clicked()
                    || (find_response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))))
            {
                let source = &state.workspace.documents()[active_index].text;
                let start = self.cursor_byte.min(source.len());
                let found = source[start..].find(&self.search).map(|offset| start + offset)
                    .or_else(|| source[..start].find(&self.search));
                if let Some(offset) = found {
                    self.cursor_byte = offset;
                    self.target_line = Some(line_for_offset(source, offset));
                    self.active_line = line_for_offset(source, offset);
                }
            }
            if ui.button("Replace next").clicked() && !self.search.is_empty() {
                let document = &mut state.workspace.documents_mut()[active_index];
                let start = self.cursor_byte.min(document.text.len());
                if let Some(relative) = document.text[start..].find(&self.search) {
                    let found = start + relative;
                    let mut text = document.text.clone();
                    text.replace_range(found..found + self.search.len(), &self.replace);
                    document.set_text(text);
                    self.cursor_byte = found + self.replace.len();
                    self.last_edit = Some(Instant::now());
                }
            }
            if ui.button("Replace all").clicked() && !self.search.is_empty() {
                let document = &mut state.workspace.documents_mut()[active_index];
                let text = document.text.replace(&self.search, &self.replace);
                document.set_text(text);
                self.last_edit = Some(Instant::now());
            }
            if ui.button("Close").clicked() { self.show_search = false; }
        });
    }

}
