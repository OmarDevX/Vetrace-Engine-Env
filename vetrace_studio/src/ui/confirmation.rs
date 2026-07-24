use super::*;

impl StudioEguiTool {
    pub(super) fn confirmation_ui(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirmation else { return; };
        let title = match action {
            Confirmation::Reload => "Reload scene?",
            Confirmation::ProjectManager => "Return to project manager?",
            Confirmation::Quit => "Quit Vetrace Studio?",
        };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("The current scene has unsaved changes.");
                ui.horizontal(|ui| {
                    if ui.button("Save and continue").clicked() {
                        self.command(match action {
                            Confirmation::Reload => StudioCommand::SaveAndReload,
                            Confirmation::ProjectManager => StudioCommand::SaveAndOpenProjectManager,
                            Confirmation::Quit => StudioCommand::SaveAndQuit,
                        });
                        self.confirmation = None;
                    }
                    if ui.button("Discard changes").clicked() {
                        self.command(match action {
                            Confirmation::Reload => StudioCommand::ReloadSceneDiscard,
                            Confirmation::ProjectManager => StudioCommand::OpenProjectManagerDiscard,
                            Confirmation::Quit => StudioCommand::QuitDiscard,
                        });
                        self.confirmation = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.confirmation = None;
                    }
                });
            });
    }
}
