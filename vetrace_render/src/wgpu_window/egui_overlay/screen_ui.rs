use super::*;

// Split-out screen-space egui drawing helpers for `wgpu_window.rs`.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    pub(super) fn render_screen_ui(ctx: &egui::Context, frame: &RenderFrame, size_in_pixels: [u32; 2], pixels_per_point: f32) {
        let ppp = pixels_per_point.max(0.001);
        let width = size_in_pixels[0].max(1) as f32;
        let height = size_in_pixels[1].max(1) as f32;
        let mut elements = frame.screen_ui.iter().collect::<Vec<_>>();
        elements.sort_by_key(|element| element.rect.z_order);

        for element in elements {
            let center = Vec2::new(element.rect.anchor.x * width, element.rect.anchor.y * height) + element.rect.offset_px;
            let size_px = Self::ui_kind_size_px(&element.kind, element.rect.size_px).max(Vec2::splat(1.0));
            let min_px = center - size_px * 0.5;
            let pos = egui::pos2(min_px.x / ppp, min_px.y / ppp);
            let size = egui::vec2(size_px.x / ppp, size_px.y / ppp);

            egui::Area::new(egui::Id::new(("vetrace_screen_ui", element.entity.0, element.slot)))
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .interactable(false)
                .show(ctx, |ui| {
                    ui.set_min_size(size);
                    Self::draw_ui_kind(ui, &element.kind, size, frame, None, Some(element.style));
                });
        }
    }

    pub(super) fn draw_ui_kind(
        ui: &mut egui::Ui,
        kind: &crate::backend::RenderScreenUiKind,
        size: egui::Vec2,
        frame: &RenderFrame,
        label_world_style: Option<(Vec3, f32, Vec2)>,
        visual_style: Option<crate::backend::RenderUiVisualStyle>,
    ) {
        let style = visual_style.unwrap_or_else(|| vetrace_ui::UIVisualStyle::default().into());
        match kind {
            crate::backend::RenderScreenUiKind::Label { text, font_size, color, align } => {
                let text_color = Self::egui_color(*color, 1.0);
                if let Some((background, background_alpha, padding_px)) = label_world_style {
                    egui::Frame::none()
                        .fill(Self::egui_color(background, background_alpha))
                        .rounding(egui::Rounding::same(style.corner_radius.max(0.0)))
                        .inner_margin(egui::Margin::symmetric(padding_px.x.max(0.0), padding_px.y.max(0.0)))
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new(text.as_str()).size(font_size.max(1.0)).color(text_color));
                        });
                } else {
                    let layout = match align {
                        vetrace_ui::TextAlign::Left => egui::Layout::left_to_right(egui::Align::Center),
                        vetrace_ui::TextAlign::Center => egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        vetrace_ui::TextAlign::Right => egui::Layout::right_to_left(egui::Align::Center),
                    };
                    ui.allocate_ui_with_layout(size, layout, |ui| {
                        ui.label(egui::RichText::new(text.as_str()).size(font_size.max(1.0)).color(text_color));
                    });
                }
            }
            crate::backend::RenderScreenUiKind::Panel { background, alpha, .. } => {
                let rect = egui::Rect::from_min_size(ui.min_rect().min, size);
                if style.shadow_alpha > 0.0 {
                    ui.painter().rect_filled(
                        rect.translate(egui::vec2(style.shadow_offset.x, style.shadow_offset.y)),
                        egui::Rounding::same(style.corner_radius.max(0.0)),
                        Self::egui_color(style.shadow_color, style.shadow_alpha),
                    );
                }
                egui::Frame::none()
                    .fill(Self::egui_color(*background, *alpha))
                    .stroke(egui::Stroke::new(style.border_width.max(0.0), Self::egui_color(style.border_color, style.border_alpha)))
                    .rounding(egui::Rounding::same(style.corner_radius.max(0.0)))
                    .show(ui, |ui| {
                        ui.set_min_size(size);
                        ui.allocate_space(size);
                    });
            }
            crate::backend::RenderScreenUiKind::Button { text, background, alpha, enabled, hovered, pressed, .. } => {
                let fill = if !*enabled {
                    Self::egui_color(background.lerp(Vec3::splat(0.28), 0.45), *alpha)
                } else if *pressed {
                    Self::egui_color(background.lerp(Vec3::ZERO, style.pressed_darkness.clamp(0.0, 1.0)), *alpha)
                } else if *hovered {
                    Self::egui_color(background.lerp(Vec3::ONE, style.hover_brightness.clamp(0.0, 1.0)), *alpha)
                } else {
                    Self::egui_color(*background, *alpha)
                };
                let text_color = if *enabled { Self::egui_color(style.text_color, style.text_alpha) } else { egui::Color32::from_gray(165) };
                let rect = egui::Rect::from_min_size(ui.min_rect().min, size);
                if style.shadow_alpha > 0.0 {
                    ui.painter().rect_filled(
                        rect.translate(egui::vec2(style.shadow_offset.x, style.shadow_offset.y)),
                        egui::Rounding::same(style.corner_radius.max(0.0)),
                        Self::egui_color(style.shadow_color, style.shadow_alpha),
                    );
                }
                egui::Frame::none()
                    .fill(fill)
                    .stroke(egui::Stroke::new(style.border_width.max(0.0), Self::egui_color(style.border_color, style.border_alpha)))
                    .rounding(egui::Rounding::same(style.corner_radius.max(0.0)))
                    .show(ui, |ui| {
                        ui.set_min_size(size);
                        ui.allocate_ui_with_layout(size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(egui::RichText::new(text.as_str()).strong().size(style.font_size.max(1.0)).color(text_color));
                        });
                    });
            }
            crate::backend::RenderScreenUiKind::TextEditor { text, placeholder, background, alpha, focused, multiline, .. } => {
                let display = if text.is_empty() { placeholder.as_str() } else { text.as_str() };
                let color = if text.is_empty() { egui::Color32::from_gray(160) } else { egui::Color32::WHITE };
                let cursor = if *focused && ((frame.settings.time_seconds * 2.0) as i32) % 2 == 0 { "|" } else { "" };
                egui::Frame::none()
                    .fill(Self::egui_color(*background, *alpha))
                    .stroke(egui::Stroke::new(style.border_width.max(0.0), Self::egui_color(style.border_color, style.border_alpha)))
                    .rounding(egui::Rounding::same(style.corner_radius.max(0.0)))
                    .show(ui, |ui| {
                        ui.set_min_size(size);
                        if *multiline {
                            ui.label(egui::RichText::new(format!("{display}{cursor}")).size(16.0).color(color));
                        } else {
                            ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.label(egui::RichText::new(format!("{display}{cursor}")).size(17.0).color(color));
                            });
                        }
                    });
            }
            crate::backend::RenderScreenUiKind::List { items, selected, .. } => {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(18, 20, 26, 220))
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_min_size(size);
                        egui::ScrollArea::vertical().max_height(size.y).show(ui, |ui| {
                            for (index, item) in items.iter().enumerate() {
                                let text = if Some(index) == *selected {
                                    egui::RichText::new(item.as_str()).strong().color(egui::Color32::WHITE)
                                } else {
                                    egui::RichText::new(item.as_str()).color(egui::Color32::from_gray(210))
                                };
                                ui.label(text);
                            }
                        });
                    });
            }
            crate::backend::RenderScreenUiKind::ColorRect { color, alpha, .. } => {
                let rect = egui::Rect::from_min_size(ui.min_rect().min, size);
                ui.painter().rect_filled(rect, egui::Rounding::same(style.corner_radius.max(0.0)), Self::egui_color(*color, *alpha));
                if style.border_width > 0.0 && style.border_alpha > 0.0 {
                    ui.painter().rect_stroke(rect, egui::Rounding::same(style.corner_radius.max(0.0)), egui::Stroke::new(style.border_width, Self::egui_color(style.border_color, style.border_alpha)));
                }
                ui.allocate_space(size);
            }
        }
    }

    pub(super) fn ui_kind_size_px(kind: &crate::backend::RenderScreenUiKind, fallback: Vec2) -> Vec2 {
        if fallback.x > 0.0 && fallback.y > 0.0 {
            return fallback;
        }
        match kind {
            crate::backend::RenderScreenUiKind::Label { .. } => Vec2::ZERO,
            crate::backend::RenderScreenUiKind::Panel { size_px, .. } => *size_px,
            crate::backend::RenderScreenUiKind::Button { size_px, .. } => *size_px,
            crate::backend::RenderScreenUiKind::TextEditor { size_px, .. } => *size_px,
            crate::backend::RenderScreenUiKind::List { size_px, .. } => *size_px,
            crate::backend::RenderScreenUiKind::ColorRect { size_px, .. } => *size_px,
        }
    }
}
