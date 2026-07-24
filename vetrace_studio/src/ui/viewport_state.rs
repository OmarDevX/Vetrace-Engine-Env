use super::*;

impl StudioEguiTool {
    pub(super) fn publish_viewport_and_capture_state(
        &mut self,
        ctx: &egui::Context,
        frame: &EguiToolContext,
        top: egui::Rect,
        left: egui::Rect,
        right: egui::Rect,
        bottom: egui::Rect,
    ) {
        let points_width = frame.screen_size_points.x.max(1.0);
        let points_height = frame.screen_size_points.y.max(1.0);
        let pixels_per_point_x = frame.surface_size_pixels[0].max(1) as f32 / points_width;
        let pixels_per_point_y = frame.surface_size_pixels[1].max(1) as f32 / points_height;
        let viewport_min_x = left.max.x * pixels_per_point_x;
        let viewport_min_y = top.max.y * pixels_per_point_y;
        let viewport = StudioViewportRect {
            min_x: viewport_min_x,
            min_y: viewport_min_y,
            max_x: (right.min.x * pixels_per_point_x).max(viewport_min_x),
            max_y: (bottom.min.y * pixels_per_point_y).max(viewport_min_y),
        };
        if let Ok(mut shared_viewport) = self.bridge.viewport_rect.lock() {
            *shared_viewport = Some(viewport);
        }

        #[cfg(feature = "render_2d")]
        self.handle_2d_texture_drop(
            ctx,
            egui::Rect::from_min_max(
                egui::pos2(left.max.x, top.max.y),
                egui::pos2(right.min.x, bottom.min.y),
            ),
            pixels_per_point_x,
            pixels_per_point_y,
        );

        if let Ok(mut captured) = self.bridge.pointer_captured.lock() {
            *captured = ctx.wants_pointer_input();
        }
        if let Ok(mut captured) = self.bridge.keyboard_captured.lock() {
            *captured = ctx.wants_keyboard_input();
        }
    }
}


#[cfg(feature = "render_2d")]
impl StudioEguiTool {
    fn handle_2d_texture_drop(
        &mut self,
        ctx: &egui::Context,
        viewport_points: egui::Rect,
        pixels_per_point_x: f32,
        pixels_per_point_y: f32,
    ) {
        if !ctx.input(|input| input.pointer.any_released()) {
            return;
        }
        let is_2d = self
            .bridge
            .snapshot
            .lock()
            .map(|snapshot| snapshot.viewport_mode == vetrace_editor::EditorViewportMode::TwoD)
            .unwrap_or(false);
        if !is_2d {
            return;
        }
        let Some(pointer) = ctx.input(|input| input.pointer.latest_pos()) else { return; };
        if !viewport_points.contains(pointer) {
            return;
        }
        let can_drop = self
            .script_assets
            .dragged_asset_mut()
            .as_ref()
            .is_some_and(|asset| asset.kind == vetrace_asset::AssetKind::Texture);
        if !can_drop {
            return;
        }
        let asset = self
            .script_assets
            .dragged_asset_mut()
            .take()
            .expect("2D texture drop checked");
        self.bridge.push(StudioCommand::SpawnSprite2DFromAsset {
            path: asset.path,
            screen_position: [
                pointer.x * pixels_per_point_x,
                pointer.y * pixels_per_point_y,
            ],
        });
    }
}
