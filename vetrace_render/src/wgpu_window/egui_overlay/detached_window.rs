use super::*;
use std::sync::Arc;

#[cfg(all(feature = "egui_render", feature = "profiler"))]
pub(super) struct DetachedProfilerWindow {
    pub(super) window: Arc<Window>,
    pub(super) surface: wgpu::Surface<'static>,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) egui_ctx: egui::Context,
    pub(super) egui_renderer: egui_wgpu::Renderer,
    pub(super) sort_mode: u8,
    pub(super) include_profiler_overhead: bool,
    pub(super) modifiers: egui::Modifiers,
    pub(super) pointer_pos: Option<egui::Pos2>,
    pub(super) pending_events: Vec<egui::Event>,
    pub(super) close_requested: bool,
    pub(super) needs_reconfigure: bool,
}

#[cfg(all(feature = "egui_render", feature = "profiler"))]
impl DetachedProfilerWindow {
    pub(super) fn new(
        event_loop: &EventLoop<()>,
        instance: &wgpu::Instance,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        _present_mode: wgpu::PresentMode,
        alpha_mode: wgpu::CompositeAlphaMode,
    ) -> Result<Self, String> {
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Vetrace Profiler")
                .with_inner_size(PhysicalSize::new(720, 860))
                .with_visible(true)
                .build(event_loop)
                .map_err(|err| err.to_string())?,
        );
        window.set_cursor_visible(true);
        let _ = window.set_cursor_grab(CursorGrabMode::None);

        let surface = instance.create_surface(window.clone()).map_err(|err| format!("failed to create profiler WGPU surface: {err}"))?;
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            // Fifo is safest for a tooling window; using the game window mode can
            // also work, but a profiler must not block normal desktop interaction.
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(device, &config);

        Ok(Self {
            window,
            surface,
            config,
            egui_ctx: egui::Context::default(),
            egui_renderer: egui_wgpu::Renderer::new(device, format, None, 1),
            sort_mode: 0,
            include_profiler_overhead: false,
            modifiers: egui::Modifiers::default(),
            pointer_pos: None,
            pending_events: Vec::new(),
            close_requested: false,
            needs_reconfigure: false,
        })
    }

    pub(super) fn id(&self) -> WindowId { self.window.id() }

    pub(super) fn handle_window_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        if window_id != self.id() { return; }
        match event {
            WindowEvent::CloseRequested => self.close_requested = true,
            WindowEvent::Resized(size) => {
                self.config.width = size.width.max(1);
                self.config.height = size.height.max(1);
                self.needs_reconfigure = true;
                self.window.request_redraw();
            }
            WindowEvent::Focused(false) => {
                self.pending_events.push(egui::Event::PointerGone);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.modifiers = egui::Modifiers {
                    alt: state.alt_key(),
                    ctrl: state.control_key(),
                    shift: state.shift_key(),
                    mac_cmd: state.super_key(),
                    command: if cfg!(target_os = "macos") { state.super_key() } else { state.control_key() },
                };
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.scale_factor() as f32;
                let pos = egui::pos2(position.x as f32 / scale.max(0.001), position.y as f32 / scale.max(0.001));
                self.pointer_pos = Some(pos);
                self.pending_events.push(egui::Event::PointerMoved(pos));
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_pos = None;
                self.pending_events.push(egui::Event::PointerGone);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(pos) = self.pointer_pos else { return; };
                let Some(button) = profiler_pointer_button(*button) else { return; };
                self.pending_events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: *state == ElementState::Pressed,
                    modifiers: self.modifiers,
                });
            }
            _ => {}
        }
    }

    pub(super) fn render(&mut self, report: &vetrace_profiler::ProfilerReport, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<(), String> {
        self.window.request_redraw();
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 { return Ok(()); }
        if self.needs_reconfigure || size.width != self.config.width || size.height != self.config.height {
            self.config.width = size.width.max(1);
            self.config.height = size.height.max(1);
            self.surface.configure(device, &self.config);
            self.needs_reconfigure = false;
        }

        let surface_texture = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(device, &self.config);
                return Ok(());
            }
            Err(wgpu::SurfaceError::Timeout) => return Ok(()),
            Err(wgpu::SurfaceError::OutOfMemory) => return Err("profiler surface out of memory".to_string()),
        };
        let surface_texture_size = surface_texture.texture.size();
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Use the window scale only for converting raw winit input into egui
        // points before egui::Context::run. The render/tessellation scale below
        // must use FullOutput::pixels_per_point to match egui's font atlas.
        let input_pixels_per_point = self.window.scale_factor() as f32;
        // Use the acquired swapchain texture size, not only the requested
        // window/config size. During live resize some platforms can hand us one
        // frame from the previous surface extent; egui_wgpu sets a viewport from
        // ScreenDescriptor, so using a larger requested size can panic WGPU with
        // "viewport is not contained in the render target".
        let size_in_pixels = [surface_texture_size.width.max(1), surface_texture_size.height.max(1)];
        let screen_size = egui::vec2(
            size_in_pixels[0] as f32 / input_pixels_per_point.max(0.001),
            size_in_pixels[1] as f32 / input_pixels_per_point.max(0.001),
        );
        // Keep egui's internal point-to-pixel scale in sync with this window.
        // The detached profiler can be dragged between monitors with different
        // DPI, so update before each egui pass instead of assuming it is static.
        self.egui_ctx.set_pixels_per_point(input_pixels_per_point.max(0.001));

        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, screen_size)),
            predicted_dt: 1.0 / 60.0,
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.pending_events),
            ..Default::default()
        };
        if let Some(pos) = self.pointer_pos {
            raw_input.events.push(egui::Event::PointerMoved(pos));
        }

        let ui_started = Instant::now();
        let mut sort_mode = self.sort_mode;
        let mut include_profiler_overhead = self.include_profiler_overhead;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Vetrace Profiler");
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    WgpuRenderer::render_profiler_contents(ui, report, &mut sort_mode, &mut include_profiler_overhead);
                });
            });
        });
        self.sort_mode = sort_mode;
        self.include_profiler_overhead = include_profiler_overhead;
        vetrace_profiler::record_timing("profiler.self.ui_run", ui_started.elapsed());

        let textures_started = Instant::now();
        for (texture_id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *texture_id, image_delta);
        }

        vetrace_profiler::record_timing("profiler.self.update_textures", textures_started.elapsed());

        let encode_started = Instant::now();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("vetrace detached profiler encoder") });
        let output_pixels_per_point = full_output.pixels_per_point;
        let screen_desc = ScreenDescriptor {
            size_in_pixels,
            pixels_per_point: output_pixels_per_point,
        };
        let primitives = self.egui_ctx.tessellate(full_output.shapes, output_pixels_per_point);
        self.egui_renderer.update_buffers(device, queue, &mut encoder, &primitives, &screen_desc);

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("vetrace detached profiler pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.018, g: 0.018, b: 0.022, a: 1.0 }), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.egui_renderer.render(&mut pass, &primitives, &screen_desc);
        }

        for texture_id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(texture_id);
        }
        vetrace_profiler::record_timing("profiler.self.encode_render_pass", encode_started.elapsed());

        let submit_started = Instant::now();
        queue.submit(Some(encoder.finish()));
        vetrace_profiler::record_timing("profiler.self.queue_submit", submit_started.elapsed());

        let present_started = Instant::now();
        surface_texture.present();
        vetrace_profiler::record_timing("profiler.self.present", present_started.elapsed());
        Ok(())
    }
}

#[cfg(all(feature = "egui_render", feature = "profiler"))]
pub(super) fn profiler_pointer_button(button: MouseButton) -> Option<egui::PointerButton> {
    Some(match button {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
        _ => return None,
    })
}
