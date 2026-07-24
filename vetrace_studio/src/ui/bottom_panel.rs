use super::*;

impl StudioEguiTool {
    pub(super) fn draw_bottom_panel(
        &mut self,
        ctx: &egui::Context,
        frame: &EguiToolContext,
        snapshot: &StudioSnapshot,
    ) -> egui::InnerResponse<()> {
        let default_height: f32 = if self.bottom_tab == BottomTab::Scripts {
            360.0
        } else {
            180.0
        };
        let max_height = (frame.screen_size_points.y * 0.62)
            .max(180.0)
            .min((frame.screen_size_points.y - 140.0).max(180.0));
        egui::TopBottomPanel::bottom("vetrace_studio_bottom")
            .default_height(default_height.min(max_height))
            .min_height(110.0)
            .max_height(max_height)
            .resizable(true)
            .show(ctx, |ui| {
                self.draw_bottom_tabs(ui, snapshot);
                ui.separator();
                self.draw_bottom_content(ui, snapshot);
            })
    }

    fn draw_bottom_tabs(&mut self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Assets, "Assets");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Console, "Console");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Scripts, "Scripts");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Project, "Project");
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Build, "Build");
            ui.separator();
            ui.label(&snapshot.status);
        });
    }

    fn draw_bottom_content(&mut self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        match self.bottom_tab {
            BottomTab::Assets => {
                for command in self.asset_browser.ui(
                    ui,
                    snapshot,
                    self.script_assets.dragged_asset_mut(),
                ) {
                    self.command(command);
                }
            }
            BottomTab::Console => self.console_ui(ui, snapshot),
            BottomTab::Scripts => {
                for command in self.script_panel.ui(ui, snapshot, &self.scripts) {
                    self.command(command);
                }
            }
            BottomTab::Project => self.project_ui(ui, snapshot),
            BottomTab::Build => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for command in self.build_panel.ui(ui, &snapshot.builds) {
                        self.command(command);
                    }
                });
            }
        }
    }
}
