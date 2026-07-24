use super::*;

impl ScriptEditorPanel {
    pub(super) fn toolbar_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut StudioScriptState,
        active_index: usize,
        snapshot: &StudioSnapshot,
        commands: &mut Vec<StudioCommand>,
    ) {
        let save_shortcut = ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::S));
        let find_shortcut = ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::F));
        if find_shortcut { self.show_search = true; }
        ui.horizontal(|ui| {
            let dirty = state.workspace.documents()[active_index].is_dirty();
            if ui.add_enabled(dirty, egui::Button::new("Save")).clicked() || save_shortcut {
                commands.push(StudioCommand::SaveScript(active_index));
            }
            if ui.button("Format").on_hover_text("Format the active Lua document").clicked() {
                let language_id = state.workspace.documents()[active_index].language_id.clone();
                if let Some(service) = state.workspace.registry().get(&language_id) {
                    let source = state.workspace.documents()[active_index].text.clone();
                    if let Ok(formatted) = service.format(&source) {
                        state.workspace.documents_mut()[active_index].set_text(formatted);
                        let _ = state.workspace.analyze(active_index);
                    }
                }
            }
            if ui.button("Find").on_hover_text("Ctrl+F").clicked() {
                self.show_search = !self.show_search;
            }
            if ui.button("Complete").on_hover_text("Ctrl+Space").clicked() {
                self.completion_open = true;
            }
            if ui.button("Definition").on_hover_text("Go to definition").clicked() {
                self.go_to_definition(state, active_index);
            }
            if ui.button("References").clicked() {
                self.find_references(state, active_index);
            }
            if ui.button("Rename").on_hover_text("Rename symbol").clicked() {
                self.show_rename_symbol = !self.show_rename_symbol;
                self.show_file_actions = false;
            }
            if ui.button("Symbols").clicked() { self.show_outline = !self.show_outline; }
            if ui.button("File…").clicked() {
                self.show_file_actions = !self.show_file_actions;
                self.show_rename_symbol = false;
                if self.rename_file_path.is_empty() {
                    self.rename_file_path = project_relative_script(&state.workspace.documents()[active_index].path);
                }
            }
            if ui.button("Breakpoint").on_hover_text("Toggle breakpoint on the current line").clicked() {
                commands.push(StudioCommand::ToggleBreakpoint {
                    path: state.workspace.documents()[active_index].path.clone(),
                    line: self.active_line,
                });
            }
            if snapshot.debugger.paused.is_some() {
                ui.colored_label(egui::Color32::YELLOW, "Paused");
            }
            ui.separator();
            let document = &state.workspace.documents()[active_index];
            let errors = document.diagnostics.iter()
                .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
                .count();
            if errors == 0 {
                ui.label(egui::RichText::new("Lua syntax valid").color(egui::Color32::from_rgb(90, 190, 110)));
            } else {
                ui.label(egui::RichText::new(format!("{errors} syntax error(s)")).color(egui::Color32::from_rgb(235, 90, 90)));
            }
            ui.separator();
            ui.label(format!("Ln {}, Col {}", self.active_line, byte_column(&document.text, self.cursor_byte)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.monospace(document.path.display().to_string());
            });
        });
    }

}
