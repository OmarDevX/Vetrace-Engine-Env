use super::*;

// Split-out world-space egui drawing helpers for `wgpu_window.rs`.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    pub(super) fn render_world_ui(ctx: &egui::Context, frame: &RenderFrame, size_in_pixels: [u32; 2], pixels_per_point: f32) {
        let view_proj = camera_matrix_for_surface(frame, size_in_pixels[0], size_in_pixels[1]);
        let ppp = pixels_per_point.max(0.001);
        let camera_position = frame.camera.position;
        let mut elements = frame.world_ui.iter().collect::<Vec<_>>();
        elements.sort_by(|a, b| {
            a.placement
                .z_order
                .cmp(&b.placement.z_order)
                .then_with(|| {
                    let da = camera_position.distance_squared(a.world_position);
                    let db = camera_position.distance_squared(b.world_position);
                    db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        for element in elements {
            if element.placement.max_distance > 0.0 && camera_position.distance(element.world_position) > element.placement.max_distance {
                continue;
            }

            let Some(mut pos) = Self::project_world_ui_position(element.world_position, view_proj, size_in_pixels, ppp) else {
                continue;
            };
            pos.x += element.placement.screen_offset_px.x / ppp;
            pos.y += element.placement.screen_offset_px.y / ppp;

            let size_px = Self::ui_kind_size_px(&element.kind, element.placement.size_px);
            let size = if size_px.x > 0.0 && size_px.y > 0.0 {
                Some(egui::vec2(size_px.x / ppp, size_px.y / ppp))
            } else {
                None
            };

            egui::Area::new(egui::Id::new(("vetrace_world_ui", element.entity.0, element.slot)))
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .pivot(Self::anchor_to_align2(element.placement.anchor))
                .interactable(false)
                .show(ctx, |ui| {
                    if let Some(size) = size {
                        ui.set_min_size(size);
                    }
                    let fallback = Some((
                        element.placement.background,
                        element.placement.background_alpha,
                        element.placement.padding_px,
                    ));
                    Self::draw_ui_kind(ui, &element.kind, size.unwrap_or(egui::Vec2::ZERO), frame, fallback, None);
                });
        }
    }

    pub(super) fn anchor_to_align2(anchor: vetrace_ui::Anchor) -> egui::Align2 {
        match anchor {
            vetrace_ui::Anchor::TopLeft => egui::Align2::LEFT_TOP,
            vetrace_ui::Anchor::TopCenter => egui::Align2::CENTER_TOP,
            vetrace_ui::Anchor::TopRight => egui::Align2::RIGHT_TOP,
            vetrace_ui::Anchor::CenterLeft => egui::Align2::LEFT_CENTER,
            vetrace_ui::Anchor::Center => egui::Align2::CENTER_CENTER,
            vetrace_ui::Anchor::CenterRight => egui::Align2::RIGHT_CENTER,
            vetrace_ui::Anchor::BottomLeft => egui::Align2::LEFT_BOTTOM,
            vetrace_ui::Anchor::BottomCenter => egui::Align2::CENTER_BOTTOM,
            vetrace_ui::Anchor::BottomRight => egui::Align2::RIGHT_BOTTOM,
        }
    }

    pub(super) fn project_world_ui_position(world_position: Vec3, view_proj: Mat4, size_in_pixels: [u32; 2], pixels_per_point: f32) -> Option<egui::Pos2> {
        let clip = view_proj * world_position.extend(1.0);
        if !clip.w.is_finite() || clip.w <= 0.0 {
            return None;
        }

        let ndc = clip.truncate() / clip.w;
        if !ndc.x.is_finite() || !ndc.y.is_finite() || !ndc.z.is_finite() {
            return None;
        }
        // Keep a small margin so projected UI does not flicker at the exact edge.
        if ndc.x < -1.15 || ndc.x > 1.15 || ndc.y < -1.15 || ndc.y > 1.15 || ndc.z < -1.15 || ndc.z > 1.15 {
            return None;
        }

        let width = size_in_pixels[0].max(1) as f32;
        let height = size_in_pixels[1].max(1) as f32;
        let x_px = (ndc.x * 0.5 + 0.5) * width;
        let y_px = (1.0 - (ndc.y * 0.5 + 0.5)) * height;
        Some(egui::pos2(x_px / pixels_per_point, y_px / pixels_per_point))
    }
}
