use super::*;

impl ScriptEditorPanel {
    pub(super) fn completion_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut StudioScriptState,
        active_index: usize,
        language_context: &LanguageContext,
    ) {
        let document = &state.workspace.documents()[active_index];
        let Some(service) = state.workspace.registry().get(&document.language_id) else { return; };
        let items = service.completions(CompletionContext {
            source: &document.text,
            cursor_byte: self.cursor_byte.min(document.text.len()),
            language: language_context,
        });
        if items.is_empty() {
            self.completion_open = false;
            return;
        }
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Completions");
                if ui.small_button("Close").clicked() { self.completion_open = false; }
            });
            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                for item in items.into_iter().take(40) {
                    if ui.selectable_label(false, format!("{}    {}", item.label, item.detail)).clicked() {
                        let document = &mut state.workspace.documents_mut()[active_index];
                        let cursor = self.cursor_byte.min(document.text.len());
                        let mut text = document.text.clone();
                        text.insert_str(cursor, &item.insert_text);
                        document.set_text(text);
                        self.cursor_byte = cursor + item.insert_text.len();
                        self.last_edit = Some(Instant::now());
                        self.completion_open = false;
                    }
                }
            });
        });
    }

}
