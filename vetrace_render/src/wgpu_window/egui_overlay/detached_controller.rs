use super::*;

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    #[cfg(all(feature = "egui_render", feature = "profiler"))]
    pub(super) fn sync_detached_profiler_window(&mut self, engine: &Engine) {
        let requested = engine
            .get_resource::<vetrace_profiler::ProfilerUiSettings>()
            .map(|settings| matches!(settings.mode, vetrace_profiler::ProfilerUiMode::Detached | vetrace_profiler::ProfilerUiMode::Both))
            .unwrap_or(false)
            && engine.contains_resource::<vetrace_profiler::ProfilerReport>();

        if !requested {
            self.optional.detached_profiler = None;
            self.optional.detached_profiler_closed_by_user = false;
            return;
        }

        if self.optional.detached_profiler.is_none() && !self.optional.detached_profiler_closed_by_user {
            let Some(desktop) = self.desktop.as_ref() else { return; };
            match DetachedProfilerWindow::new(&desktop.event_loop, &self.core.instance, &self.core.device, self.core.surface_view_format, self.core.config.present_mode, self.core.config.alpha_mode) {
                Ok(window) => self.optional.detached_profiler = Some(window),
                Err(err) => {
                    eprintln!("vetrace_profiler: failed to create detached profiler window: {err}");
                    self.optional.detached_profiler_closed_by_user = true;
                }
            }
        }
    }

    #[cfg(all(feature = "egui_render", feature = "profiler"))]
    pub(super) fn apply_detached_profiler_event_results(&mut self) {
        if self.optional.detached_profiler.as_ref().map(|window| window.close_requested).unwrap_or(false) {
            self.optional.detached_profiler = None;
            self.optional.detached_profiler_closed_by_user = true;
        }
    }

    #[cfg(all(feature = "egui_render", feature = "profiler"))]
    pub(super) fn render_detached_profiler_window(&mut self, frame: &RenderFrame) {
        if !Self::profiler_wants_detached(frame) { return; }
        let Some(report) = frame.profiler_report.as_ref() else { return; };
        let Some(window) = self.optional.detached_profiler.as_mut() else { return; };

        let started = Instant::now();
        match window.render(report, &self.core.device, &self.core.queue) {
            Ok(()) => vetrace_profiler::record_timing("profiler.self.window_cpu_total", started.elapsed()),
            Err(err) => {
                eprintln!("vetrace_profiler: detached profiler window render failed: {err}");
                self.optional.detached_profiler = None;
                self.optional.detached_profiler_closed_by_user = true;
            }
        }
    }
}
