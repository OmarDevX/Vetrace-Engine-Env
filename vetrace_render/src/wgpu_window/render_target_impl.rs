use super::*;

// Desktop RenderTarget wiring. The rendering itself lives on WgpuRenderer and
// is also called directly by the browser adapter.

impl RenderTarget for WgpuRenderer {
    fn begin_frame(&mut self, engine: &mut Engine) {
        let Some(desktop) = self.desktop.as_ref() else { return; };
        desktop.window.request_redraw();
        self.sync_cursor_settings(engine);
        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        self.sync_detached_profiler_window(engine);

        let game_window_id = self
            .desktop
            .as_ref()
            .expect("desktop renderer state disappeared")
            .window
            .id();
        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        let detached_profiler = self.optional.detached_profiler.as_mut();
        #[cfg(not(all(feature = "egui_render", feature = "profiler")))]
        let detached_profiler: Option<&mut DetachedProfilerWindow> = None;
        if let Some(desktop) = self.desktop.as_mut() {
            pump_winit_input(
                engine,
                &mut desktop.event_loop,
                game_window_id,
                &mut desktop.game_window_focused,
                detached_profiler,
            );
        }
        #[cfg(all(feature = "egui_render", feature = "profiler"))]
        self.apply_detached_profiler_event_results();
        let (size, pixel_scale_factor) = {
            let desktop_window = &self
                .desktop
                .as_ref()
                .expect("desktop renderer state disappeared")
                .window;
            (desktop_window.inner_size(), desktop_window.scale_factor() as f32)
        };
        self.set_pixel_scale_factor(pixel_scale_factor);
        if size.width != self.core.config.width || size.height != self.core.config.height {
            self.resize(size.width, size.height);
        }
        self.sync_surface_size_resource(engine);
    }

    fn render(&mut self, frame: &RenderFrame, assets: Option<&RenderAssets>) {
        self.render_frame(frame, assets);
    }
}
