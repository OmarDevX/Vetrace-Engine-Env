use super::*;

impl ScriptEditorPanel {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &StudioSnapshot,
        scripts: &StudioScripts,
    ) -> Vec<StudioCommand> {
        let mut commands = Vec::new();
        let Some(()) = scripts.with_state(|state| {
            self.tabs_ui(ui, state);
            let Some(active_index) = state.workspace.active_index() else {
                ui.centered_and_justified(|ui| {
                    ui.label("Double-click a Lua file in Assets to open it here.");
                });
                return;
            };
            self.sync_active_view(state, active_index);
            self.external_change_ui(ui, state, active_index, &mut commands);

            if let Some(line) = state.target_line.take() {
                self.target_line = Some(line);
                self.active_line = line;
            }

            self.toolbar_ui(ui, state, active_index, snapshot, &mut commands);
            self.debugger_ui(ui, snapshot, &mut commands);
            if self.show_search { self.search_ui(ui, state, active_index); }
            if self.show_rename_symbol { self.rename_symbol_ui(ui, state, active_index); }
            if self.show_file_actions { self.file_actions_ui(ui, state, active_index, &mut commands); }
            if self.show_outline { self.outline_ui(ui, state, active_index); }
            if !self.references.is_empty() { self.references_ui(ui, state, active_index); }
            if let Some(status) = &self.status {
                ui.label(egui::RichText::new(status).small().color(egui::Color32::LIGHT_BLUE));
            }
            ui.separator();
            let available_height = ui.available_height().max(120.0);
            let diagnostics_count = StudioScripts::syntax_and_runtime_diagnostics(state, active_index).len();
            let diagnostics_height = if diagnostics_count == 0 {
                24.0
            } else {
                (available_height * 0.24).clamp(72.0, 150.0)
            };
            let editor_height = (available_height - diagnostics_height - 10.0).max(88.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), editor_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.editor_ui(ui, state, active_index, snapshot, &mut commands),
            );
            ui.separator();
            egui::ScrollArea::vertical()
                .id_source("vetrace_script_diagnostics_scroll")
                .max_height(diagnostics_height)
                .auto_shrink([false, false])
                .show(ui, |ui| self.diagnostics_ui(ui, state, active_index));

            let ctrl_space = ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Space));
            if ctrl_space { self.completion_open = true; }
            if self.completion_open {
                self.completion_ui(ui, state, active_index, &snapshot.language_context);
            }

            if let Some(index) = self.pending_close {
                self.close_confirmation_ui(ui, state, index, &mut commands);
            }
            if let Some(index) = self.pending_delete {
                self.delete_confirmation_ui(ui, state, index, &mut commands);
            }
        }) else {
            ui.label("Script editor state is unavailable.");
            return commands;
        };
        commands
    }

}
