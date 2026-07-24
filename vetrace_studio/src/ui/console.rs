use super::*;

impl StudioEguiTool {
    pub(super) fn console_ui(&self, ui: &mut egui::Ui, snapshot: &crate::protocol::StudioSnapshot) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &snapshot.logs {
                    if let Some((path, line_number)) = parse_console_script_location(&snapshot.project_root, line) {
                        if ui.selectable_label(false, egui::RichText::new(line).monospace()).clicked() {
                            self.command(StudioCommand::OpenScriptAt { path, line: line_number });
                        }
                    } else {
                        ui.monospace(line);
                    }
                }
            });
    }

}
