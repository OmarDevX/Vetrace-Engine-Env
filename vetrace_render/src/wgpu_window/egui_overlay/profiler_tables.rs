use super::*;

// Profiler timing, memory, and counter tables.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    #[cfg(feature = "profiler")]
    pub(super) fn render_profiler_overhead(ui: &mut egui::Ui, timings: &[vetrace_profiler::ProfileTiming], sort_mode: u8) {
        let mut profiler_timings = timings.iter().filter(|timing| Self::is_profiler_overhead(&timing.name)).collect::<Vec<_>>();
        if profiler_timings.is_empty() { return; }
        Self::sort_timings(&mut profiler_timings, sort_mode);
        let total = Self::profiler_overhead_ms(timings);
        egui::CollapsingHeader::new(format!("profiler.self overhead  {total:.2} ms"))
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Profiler UI/window cost is tracked separately so it does not pollute game timings by default.");
                ui.label("Inclusive parent scopes such as app.render/plugin.render.render/render.target_render are hidden from the default game view because they include detached-window work.");
                for timing in profiler_timings {
                    Self::timing_row(ui, timing, 16.667);
                }
            });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn group_total_ms(report: &vetrace_profiler::ProfilerReport, group: &str, group_timings: &[&vetrace_profiler::ProfileTiming]) -> f32 {
        match group {
            "wgpu.game" => Self::timing_latest_ms(report, "wgpu.game.frame_cpu_total")
                .unwrap_or_else(|| group_timings.iter().map(|timing| timing.total_ms).sum()),
            "profiler.self" => Self::profiler_overhead_ms(&report.timings),
            _ => group_timings.iter().map(|timing| timing.total_ms).sum(),
        }
    }

    #[cfg(feature = "profiler")]
    pub(super) fn timing_row(ui: &mut egui::Ui, timing: &vetrace_profiler::ProfileTiming, budget_ms: f32) {
        let color = Self::timing_color(timing.total_ms);
        ui.horizontal(|ui| {
            ui.add_sized([70.0, 14.0], egui::ProgressBar::new((timing.total_ms / budget_ms.max(0.001)).clamp(0.0, 1.0)).text(""));
            ui.colored_label(color, format!("{:>6.2} ms", timing.total_ms));
            ui.label(format!("x{:>2}", timing.calls));
            ui.monospace(timing.name.as_str());
        });
        ui.horizontal(|ui| {
            ui.add_space(78.0);
            ui.label(egui::RichText::new(format!(
                "avg/call {:.3} ms | call max {:.3} ms | hist avg/min/max {:.3}/{:.3}/{:.3} ms",
                timing.average_ms,
                timing.max_ms,
                timing.rolling_average_ms,
                timing.rolling_min_ms,
                timing.rolling_max_ms,
            )).small());
        });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn frame_budget_bar(ui: &mut egui::Ui, label: &str, value_ms: f32, budget_ms: f32) {
        let color = Self::timing_color(value_ms);
        ui.horizontal(|ui| {
            ui.label(format!("{label:>13}"));
            ui.add_sized([160.0, 16.0], egui::ProgressBar::new((value_ms / budget_ms.max(0.001)).clamp(0.0, 1.0)).text(format!("{value_ms:.2}/{budget_ms:.2} ms")));
            ui.colored_label(color, Self::budget_status(value_ms, budget_ms));
        });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn render_memory_breakdown(ui: &mut egui::Ui, counters: &[vetrace_profiler::ProfileCounter]) {
        let memory = [
            ("textures", Self::counter_bytes(counters, "wgpu.memory.textures_bytes")),
            ("shadow maps", Self::counter_bytes(counters, "wgpu.memory.shadow_maps_bytes")),
            ("SSAO targets", Self::counter_bytes(counters, "wgpu.memory.ssao_targets_bytes")),
            ("mesh buffers", Self::counter_bytes(counters, "wgpu.memory.mesh_buffers_bytes")),
            ("uniform buffers", Self::counter_bytes(counters, "wgpu.memory.uniform_buffers_bytes")),
        ];
        let total = Self::counter_bytes(counters, "wgpu.estimated_resource_bytes")
            .or_else(|| Some(memory.iter().filter_map(|(_, value)| *value).sum::<u64>()))
            .unwrap_or(0);
        egui::CollapsingHeader::new(format!("GPU memory breakdown  {}", Self::format_bytes(total)))
            .default_open(true)
            .show(ui, |ui| {
                for (name, value) in memory {
                    if let Some(value) = value {
                        let pct = if total > 0 { value as f32 / total as f32 } else { 0.0 };
                        ui.horizontal(|ui| {
                            ui.add_sized([110.0, 14.0], egui::ProgressBar::new(pct.clamp(0.0, 1.0)).text(format!("{:>4.0}%", pct * 100.0)));
                            ui.label(format!("{name:16} {}", Self::format_bytes(value)));
                        });
                    }
                }
                if let Some(buffer_total) = Self::counter_bytes(counters, "wgpu.memory.buffers_bytes") {
                    ui.separator();
                    ui.label(format!("buffers total   {}", Self::format_bytes(buffer_total)));
                }
            });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn render_counters(ui: &mut egui::Ui, counters: &[vetrace_profiler::ProfileCounter]) {
        egui::CollapsingHeader::new("Counters")
            .default_open(false)
            .show(ui, |ui| {
                for counter in counters {
                    if counter.name.starts_with("wgpu.memory.") { continue; }
                    ui.horizontal(|ui| {
                        ui.monospace(counter.name.as_str());
                        ui.label(Self::format_counter_value(counter));
                    });
                }
            });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn counter_bytes(counters: &[vetrace_profiler::ProfileCounter], name: &str) -> Option<u64> {
        counters.iter().find(|counter| counter.name == name).map(|counter| counter.value.max(0.0) as u64)
    }
}
