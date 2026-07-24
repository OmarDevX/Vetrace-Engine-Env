use super::*;

// Split-out implementation details for `wgpu_window.rs`.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    pub(super) fn render_egui_overlay(&mut self, frame: &RenderFrame, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let has_panel = frame.egui_overlay.as_ref().map(|panel| panel.enabled).unwrap_or(false);
        let has_world_ui = !frame.world_ui.is_empty();
        let has_screen_ui = !frame.screen_ui.is_empty();
        let has_egui_tools = frame.egui_tools.as_ref().map(|tools| !tools.is_empty()).unwrap_or(false);
        #[cfg(feature = "profiler")]
        let has_profiler_report = frame.profiler_report.is_some() && Self::profiler_wants_overlay(frame);
        #[cfg(not(feature = "profiler"))]
        let has_profiler_report = false;
        if !has_panel && !has_profiler_report && !has_world_ui && !has_screen_ui && !has_egui_tools { return; }

        // Use the window scale only to convert physical input pixels into egui points
        // before the egui pass. The actual tessellation/render scale must come from
        // FullOutput::pixels_per_point below, because egui can choose/update the font
        // atlas scale independently from the raw winit window scale on Windows/HiDPI.
        let input_pixels_per_point = self.core.pixel_scale_factor.max(0.001);
        let size_in_pixels = [self.core.config.width.max(1), self.core.config.height.max(1)];
        let screen_size = egui::vec2(
            size_in_pixels[0] as f32 / input_pixels_per_point.max(0.001),
            size_in_pixels[1] as f32 / input_pixels_per_point.max(0.001),
        );
        // Keep egui's internal point-to-pixel scale in sync with the window
        // scale before building UI. Without this, input hit-testing can happen
        // in the right logical point space while the final paint jobs are
        // rasterized with an older/default scale, which makes the visible UI
        // appear shifted or scaled on Windows HiDPI displays.
        self.optional.egui_ctx.set_pixels_per_point(input_pixels_per_point.max(0.001));

        let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size);
        let mut raw_input = egui::RawInput {
            screen_rect: Some(screen_rect),
            predicted_dt: 1.0 / 60.0,
            ..Default::default()
        };

        if let Some(input) = frame.egui_input.as_ref() {
            let pointer_pos = egui::pos2(
                input.mouse_position[0] / input_pixels_per_point.max(0.001),
                input.mouse_position[1] / input_pixels_per_point.max(0.001),
            );
            raw_input.modifiers = egui::Modifiers {
                alt: input.alt,
                ctrl: input.ctrl,
                shift: input.shift,
                mac_cmd: false,
                command: input.ctrl,
            };

            // Egui is immediate-mode, but it still needs real pointer events.
            // Feed the current pointer position every frame and frame-local
            // button transitions from vetrace_core input.
            if screen_rect.contains(pointer_pos) {
                raw_input.events.push(egui::Event::PointerMoved(pointer_pos));
            } else {
                raw_input.events.push(egui::Event::PointerGone);
            }

            super::egui_overlay_helpers::push_pointer_button_event(&mut raw_input.events, pointer_pos, egui::PointerButton::Primary, input.left_pressed, input.left_released, raw_input.modifiers);
            super::egui_overlay_helpers::push_pointer_button_event(&mut raw_input.events, pointer_pos, egui::PointerButton::Secondary, input.right_pressed, input.right_released, raw_input.modifiers);
            super::egui_overlay_helpers::push_pointer_button_event(&mut raw_input.events, pointer_pos, egui::PointerButton::Middle, input.middle_pressed, input.middle_released, raw_input.modifiers);

            let wheel = egui::vec2(input.mouse_wheel_delta[0], input.mouse_wheel_delta[1]);
            if wheel != egui::Vec2::ZERO {
                // Winit line deltas are usually small integers while trackpad
                // pixel deltas are already in a useful point-like range.
                let scale = if wheel.x.abs().max(wheel.y.abs()) <= 4.0 { 24.0 } else { 1.0 };
                raw_input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: wheel * scale,
                    modifiers: raw_input.modifiers,
                });
            }
        }

        if let Some(input) = frame.egui_keyboard_input.as_ref() {
            for event in &input.key_events {
                let Some(key) = egui_key(&event.key) else { continue; };
                raw_input.events.push(egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: event.pressed,
                    repeat: false,
                    modifiers: raw_input.modifiers,
                });
            }
            if !input.text_input.is_empty() && !raw_input.modifiers.command {
                raw_input.events.push(egui::Event::Text(input.text_input.clone()));
            }
        }

        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        let mut profiler_sort_mode = self.optional.profiler_sort_mode;
        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        let mut profiler_include_overhead = self.optional.profiler_include_overhead;

        let full_output = self.optional.egui_ctx.run(raw_input, |ctx| {
            #[cfg(feature = "profiler")]
            if Self::profiler_wants_overlay(frame) {
                if let Some(report) = frame.profiler_report.as_ref() {
                    Self::render_profiler_report_window(ctx, report, &mut profiler_sort_mode, &mut profiler_include_overhead);
                }
            }

            if has_world_ui {
                Self::render_world_ui(ctx, frame, size_in_pixels, input_pixels_per_point);
            }
            if has_screen_ui {
                Self::render_screen_ui(ctx, frame, size_in_pixels, input_pixels_per_point);
            }

            if has_egui_tools {
                if let Some(tools) = frame.egui_tools.as_ref() {
                    let tool_context = crate::resources::EguiToolContext {
                        screen_size_points: Vec2::new(screen_size.x, screen_size.y),
                        surface_size_pixels: size_in_pixels,
                        pixels_per_point: input_pixels_per_point,
                        camera: frame.camera.clone(),
                        input: frame.egui_input,
                        keyboard_input: frame.egui_keyboard_input.clone(),
                    };
                    tools.run(ctx, &tool_context);
                }
            }

            if let Some(panel) = frame.egui_overlay.as_ref() {
                #[cfg(feature = "profiler")]
                let mut draw_panel = panel.enabled;
                #[cfg(not(feature = "profiler"))]
                let draw_panel = panel.enabled;
                #[cfg(feature = "profiler")]
                if Self::profiler_wants_overlay(frame) && frame.profiler_report.is_some() && panel.title == "Vetrace Profiler" {
                    // The rich profiler window below supersedes the generic
                    // text-only fallback panel written by vetrace_profiler.
                    draw_panel = false;
                }
                if draw_panel {
                    Self::render_text_overlay_panel(ctx, panel);
                }
            }
        });

        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        {
            self.optional.profiler_sort_mode = profiler_sort_mode;
            self.optional.profiler_include_overhead = profiler_include_overhead;
        }

        for (texture_id, image_delta) in &full_output.textures_delta.set {
            self.optional.egui_renderer.update_texture(&self.core.device, &self.core.queue, *texture_id, image_delta);
        }

        let output_pixels_per_point = full_output.pixels_per_point;
        let screen_desc = ScreenDescriptor {
            size_in_pixels,
            pixels_per_point: output_pixels_per_point,
        };
        let primitives = self.optional.egui_ctx.tessellate(full_output.shapes, output_pixels_per_point);
        self.optional.egui_renderer.update_buffers(&self.core.device, &self.core.queue, encoder, &primitives, &screen_desc);

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace egui overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.optional.egui_renderer.render(&mut pass, &primitives, &screen_desc);
        }

        for texture_id in &full_output.textures_delta.free {
            self.optional.egui_renderer.free_texture(texture_id);
        }
    }
}


#[cfg(feature = "egui_render")]
pub(super) fn egui_key(name: &str) -> Option<egui::Key> {
    Some(match name {
        "ArrowDown" => egui::Key::ArrowDown,
        "ArrowLeft" => egui::Key::ArrowLeft,
        "ArrowRight" => egui::Key::ArrowRight,
        "ArrowUp" => egui::Key::ArrowUp,
        "Escape" => egui::Key::Escape,
        "Tab" => egui::Key::Tab,
        "Backspace" => egui::Key::Backspace,
        "Enter" => egui::Key::Enter,
        "Space" => egui::Key::Space,
        "Delete" => egui::Key::Delete,
        "A" => egui::Key::A,
        "B" => egui::Key::B,
        "C" => egui::Key::C,
        "D" => egui::Key::D,
        "E" => egui::Key::E,
        "F" => egui::Key::F,
        "G" => egui::Key::G,
        "H" => egui::Key::H,
        "I" => egui::Key::I,
        "J" => egui::Key::J,
        "K" => egui::Key::K,
        "L" => egui::Key::L,
        "M" => egui::Key::M,
        "N" => egui::Key::N,
        "O" => egui::Key::O,
        "P" => egui::Key::P,
        "Q" => egui::Key::Q,
        "R" => egui::Key::R,
        "S" => egui::Key::S,
        "T" => egui::Key::T,
        "U" => egui::Key::U,
        "V" => egui::Key::V,
        "W" => egui::Key::W,
        "X" => egui::Key::X,
        "Y" => egui::Key::Y,
        "Z" => egui::Key::Z,
        _ => return None,
    })
}
