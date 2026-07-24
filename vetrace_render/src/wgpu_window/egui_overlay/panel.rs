use super::*;

// Text-panel egui overlay helpers for `wgpu_window.rs`.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    pub(super) fn render_text_overlay_panel(ctx: &egui::Context, panel: &crate::resources::EguiOverlayPanel) {
        egui::Window::new(panel.title.as_str())
            .default_pos(egui::pos2(14.0, 14.0))
            .default_width(340.0)
            .resizable(true)
            .show(ctx, |ui| {
                if !panel.subtitle.is_empty() {
                    ui.label(panel.subtitle.as_str());
                    ui.separator();
                }
                if !panel.status.is_empty() {
                    ui.strong(panel.status.as_str());
                    ui.separator();
                }
                for line in &panel.lines {
                    ui.label(line.as_str());
                }
                if !panel.controls.is_empty() {
                    ui.separator();
                    ui.collapsing("Controls", |ui| {
                        for line in &panel.controls {
                            ui.label(line.as_str());
                        }
                    });
                }
            });
    }
}
