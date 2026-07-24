use super::*;

// Profiler mode selection, timing grouping, and aggregation.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    #[cfg(feature = "profiler")]
    pub(super) fn profiler_ui_mode(frame: &RenderFrame) -> vetrace_profiler::ProfilerUiMode {
        frame.profiler_ui_settings.as_ref().map(|settings| settings.mode).unwrap_or(vetrace_profiler::ProfilerUiMode::Detached)
    }

    #[cfg(feature = "profiler")]
    pub(super) fn profiler_wants_overlay(frame: &RenderFrame) -> bool {
        matches!(Self::profiler_ui_mode(frame), vetrace_profiler::ProfilerUiMode::Overlay | vetrace_profiler::ProfilerUiMode::Both)
    }

    #[cfg(feature = "profiler")]
    pub(super) fn profiler_wants_detached(frame: &RenderFrame) -> bool {
        matches!(Self::profiler_ui_mode(frame), vetrace_profiler::ProfilerUiMode::Detached | vetrace_profiler::ProfilerUiMode::Both)
    }

    #[cfg(feature = "profiler")]
    pub(super) fn sort_timings(timings: &mut [&vetrace_profiler::ProfileTiming], sort_mode: u8) {
        timings.sort_by(|a, b| {
            let av = match sort_mode {
                1 => a.average_ms,
                2 => a.rolling_average_ms,
                3 => a.rolling_max_ms,
                4 => a.calls as f32,
                _ => a.total_ms,
            };
            let bv = match sort_mode {
                1 => b.average_ms,
                2 => b.rolling_average_ms,
                3 => b.rolling_max_ms,
                4 => b.calls as f32,
                _ => b.total_ms,
            };
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn timing_group(name: &str) -> &'static str {
        if Self::is_profiler_overhead(name) {
            "profiler.self"
        } else if name.starts_with("scene.") || name.contains("scene_") || name.contains(".scene") {
            "scene"
        } else if name.starts_with("wgpu.gpu.") {
            "wgpu.gpu"
        } else if name.starts_with("wgpu.game.") || name.starts_with("wgpu.") {
            "wgpu.game"
        } else if name.starts_with("render.") || name.starts_with("plugin.render.") {
            "render"
        } else if name.starts_with("physics.") || name.contains("rapier") || name.starts_with("plugin.rapier_physics.") {
            "physics"
        } else if name.starts_with("app.") || name.starts_with("plugin.") {
            "app"
        } else {
            "other"
        }
    }

    #[cfg(feature = "profiler")]
    pub(super) fn group_label(group: &str) -> &'static str {
        match group {
            "wgpu.game" => "wgpu.game CPU/submit",
            "wgpu.gpu" => "wgpu.gpu shader/pass time",
            "profiler.self" => "profiler.self",
            "app" => "app",
            "render" => "render",
            "physics" => "physics",
            "scene" => "scene",
            _ => "other",
        }
    }

    #[cfg(feature = "profiler")]
    pub(super) fn is_profiler_overhead(name: &str) -> bool {
        name.starts_with("profiler.self.") || name.starts_with("plugin.profiler.") || name.starts_with("wgpu.profiler_window")
    }

    #[cfg(feature = "profiler")]
    pub(super) fn is_profiler_polluted_parent(name: &str) -> bool {
        // These are inclusive parent scopes around the whole render phase. With a
        // detached profiler window they include profiler.self.* work, so hide them
        // from the default game-only view. The detailed game WGPU timing is still
        // shown through wgpu.game.frame_cpu_total and the per-pass wgpu.game.* rows.
        matches!(name, "app.render" | "plugin.render.render" | "render.target_render")
    }

    #[cfg(feature = "profiler")]
    pub(super) fn timing_latest_ms(report: &vetrace_profiler::ProfilerReport, name: &str) -> Option<f32> {
        report.timings.iter().find(|timing| timing.name == name).map(|timing| timing.total_ms)
    }

    #[cfg(feature = "profiler")]
    pub(super) fn timing_average_ms(report: &vetrace_profiler::ProfilerReport, name: &str) -> Option<f32> {
        report.timings.iter().find(|timing| timing.name == name).map(|timing| timing.rolling_average_ms)
    }

    #[cfg(feature = "profiler")]
    pub(super) fn profiler_overhead_ms(timings: &[vetrace_profiler::ProfileTiming]) -> f32 {
        let window_total = timings
            .iter()
            .find(|timing| timing.name == "profiler.self.window_cpu_total")
            .map(|timing| timing.total_ms);
        let plugin_total = timings
            .iter()
            .filter(|timing| timing.name.starts_with("plugin.profiler."))
            .map(|timing| timing.total_ms)
            .sum::<f32>();
        if let Some(window_total) = window_total {
            window_total + plugin_total
        } else {
            timings.iter().filter(|timing| Self::is_profiler_overhead(&timing.name)).map(|timing| timing.total_ms).sum()
        }
    }
}
