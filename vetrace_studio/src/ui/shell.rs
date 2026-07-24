use super::*;

impl EguiTool for StudioEguiTool {
    fn ui(&mut self, ctx: &egui::Context, frame: &EguiToolContext) {
        let snapshot = self
            .bridge
            .snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default();
        if self.scripts.take_focus_request() {
            self.bottom_tab = BottomTab::Scripts;
        }

        let top_panel = self.draw_toolbar(ctx, &snapshot);
        let left_panel = self.draw_scene_tree(ctx, &snapshot);
        let right_panel = self.draw_inspector_panel(ctx, &snapshot);
        let bottom_panel = self.draw_bottom_panel(ctx, frame, &snapshot);

        self.confirmation_ui(ctx);
        for command in self.script_assets.dialog_ui(ctx, &snapshot) {
            self.command(command);
        }
        self.publish_viewport_and_capture_state(
            ctx,
            frame,
            top_panel.response.rect,
            left_panel.response.rect,
            right_panel.response.rect,
            bottom_panel.response.rect,
        );
        self.script_assets.finish_frame(ctx);
    }
}
