use super::*;

impl StudioEguiTool {
    pub(super) fn draw_scene_tree(
        &mut self,
        ctx: &egui::Context,
        snapshot: &StudioSnapshot,
    ) -> egui::InnerResponse<()> {
        egui::SidePanel::left("vetrace_studio_scene_tree")
            .default_width(230.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Scene");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for row in &snapshot.entities {
                        ui.horizontal(|ui| {
                            ui.add_space(row.depth as f32 * 14.0);
                            let selected = snapshot.selected == Some(row.entity);
                            if ui.selectable_label(selected, &row.name).clicked() {
                                self.command(StudioCommand::Select(Some(row.entity)));
                            }
                        });
                    }
                });
            })
    }
}
