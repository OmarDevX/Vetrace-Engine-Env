use super::*;

impl ScriptEditorPanel {
    pub(super) fn diagnostics_ui(&mut self, ui: &mut egui::Ui, state: &mut StudioScriptState, active_index: usize) {
        let diagnostics = StudioScripts::syntax_and_runtime_diagnostics(state, active_index);
        if diagnostics.is_empty() {
            ui.label(egui::RichText::new("No script diagnostics").small().color(egui::Color32::GRAY));
            return;
        }
        egui::CollapsingHeader::new(format!("Problems ({})", diagnostics.len()))
            .default_open(true)
            .show(ui, |ui| {
                for diagnostic in diagnostics {
                    ui.horizontal_wrapped(|ui| {
                        let icon = match diagnostic.severity {
                            DiagnosticSeverity::Error => "⛔",
                            DiagnosticSeverity::Warning => "⚠",
                            DiagnosticSeverity::Information => "ℹ",
                            DiagnosticSeverity::Hint => "💡",
                        };
                        if ui.selectable_label(false, format!(
                            "{icon} {}:{}  {}",
                            diagnostic.position.line,
                            diagnostic.position.column,
                            diagnostic.message,
                        )).clicked() {
                            self.target_line = Some(diagnostic.position.line);
                            self.active_line = diagnostic.position.line;
                        }
                        for action in &diagnostic.actions {
                            if ui.small_button(&action.title).clicked() {
                                let applied = {
                                    let document = &mut state.workspace.documents_mut()[active_index];
                                    document.apply_edits(&action.edits).is_ok()
                                };
                                if applied {
                                    let _ = state.workspace.analyze(active_index);
                                    self.target_line = Some(diagnostic.position.line);
                                }
                            }
                        }
                    });
                }
            });
    }

}
