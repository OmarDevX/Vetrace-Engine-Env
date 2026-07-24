use super::*;

// Profiler window and main contents layout.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    #[cfg(feature = "profiler")]
    pub(super) fn render_profiler_report_window(ctx: &egui::Context, report: &vetrace_profiler::ProfilerReport, sort_mode: &mut u8, include_profiler_overhead: &mut bool) {
        egui::Window::new("Vetrace Profiler")
            .default_pos(egui::pos2(14.0, 14.0))
            .default_width(440.0)
            .default_height(620.0)
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                Self::render_profiler_contents(ui, report, sort_mode, include_profiler_overhead);
            });
    }

    #[cfg(feature = "profiler")]
    pub(super) fn render_profiler_contents(ui: &mut egui::Ui, report: &vetrace_profiler::ProfilerReport, sort_mode: &mut u8, include_profiler_overhead: &mut bool) {
        ui.label("Runtime CPU/RAM/frame/WGPU profiling view");
        ui.separator();

        let profiler_overhead_ms = Self::profiler_overhead_ms(&report.timings);
        let game_latest_ms = if *include_profiler_overhead {
            report.latest_frame_ms
        } else {
            Self::timing_latest_ms(report, "wgpu.game.frame_cpu_total")
                .unwrap_or_else(|| (report.latest_frame_ms - profiler_overhead_ms).max(0.0))
        };
        let game_average_ms = if *include_profiler_overhead {
            report.average_frame_ms
        } else {
            Self::timing_average_ms(report, "wgpu.game.frame_cpu_total").unwrap_or(report.average_frame_ms)
        };
        let display_fps = if game_average_ms > 0.001 { 1000.0 / game_average_ms } else { report.fps };

        ui.horizontal(|ui| {
            ui.strong(format!("{display_fps:.1} FPS"));
            ui.label(format!("game latest {:.2} ms", game_latest_ms));
            ui.label(format!("game avg {:.2} ms", game_average_ms));
        });
        Self::frame_budget_bar(ui, "game frame", game_latest_ms, 16.667);
        Self::frame_budget_bar(ui, "game average", game_average_ms, 16.667);
        if !report.frame_history_ms.is_empty() {
            ui.monospace(format!("total frames {}", Self::sparkline(&report.frame_history_ms, 84)));
        }

        ui.horizontal_wrapped(|ui| {
            ui.checkbox(include_profiler_overhead, "include profiler overhead in main totals");
            if !*include_profiler_overhead && profiler_overhead_ms > 0.001 {
                ui.label(format!("excluded profiler overhead {:.2} ms", profiler_overhead_ms));
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.label(format!("cpu {}", report.process_cpu_percent.map(|v| format!("{v:.1}%")).unwrap_or_else(|| "n/a".to_string())));
            let process_ram = report.process_memory_bytes.map(Self::format_bytes).unwrap_or_else(|| "n/a".to_string());
            let system_ram = match (report.system_memory_available_bytes, report.system_memory_total_bytes) {
                (Some(avail), Some(total)) => format!("{} free / {} total", Self::format_bytes(avail), Self::format_bytes(total)),
                _ => "system n/a".to_string(),
            };
            ui.label(format!("ram {process_ram} ({system_ram})"));
        });

        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("Sort timings:");
            ui.radio_value(sort_mode, 0, "latest");
            ui.radio_value(sort_mode, 1, "avg/call");
            ui.radio_value(sort_mode, 2, "history avg");
            ui.radio_value(sort_mode, 3, "history max");
            ui.radio_value(sort_mode, 4, "calls");
        });

        let mut timings = report
            .timings
            .iter()
            .filter(|timing| {
                *include_profiler_overhead
                    || (!Self::is_profiler_overhead(&timing.name) && !Self::is_profiler_polluted_parent(&timing.name))
            })
            .collect::<Vec<_>>();
        Self::sort_timings(&mut timings, *sort_mode);

        for group in ["app", "render", "physics", "wgpu.game", "wgpu.gpu", "scene", "other", "profiler.self"] {
            if group == "profiler.self" && !*include_profiler_overhead { continue; }
            let group_timings = timings.iter().copied().filter(|timing| Self::timing_group(&timing.name) == group).collect::<Vec<_>>();
            if group_timings.is_empty() { continue; }
            let group_total = Self::group_total_ms(report, group, &group_timings);
            let title = format!("{}  {:.2} ms", Self::group_label(group), group_total);
            egui::CollapsingHeader::new(title)
                .default_open(matches!(group, "render" | "wgpu.game" | "wgpu.gpu" | "physics"))
                .show(ui, |ui| {
                    for timing in group_timings {
                        Self::timing_row(ui, timing, 16.667);
                    }
                });
        }

        Self::render_profiler_overhead(ui, &report.timings, *sort_mode);

        Self::render_memory_breakdown(ui, &report.counters);
        Self::render_counters(ui, &report.counters);

        egui::CollapsingHeader::new("GPU timestamp queries")
            .default_open(true)
            .show(ui, |ui| {
                let supported = report.counters.iter().find(|counter| counter.name == "wgpu.gpu.timestamp_queries_supported").map(|counter| counter.value > 0.5).unwrap_or(false);
                let enabled = report.counters.iter().find(|counter| counter.name == "wgpu.gpu.timestamp_queries_enabled").map(|counter| counter.value > 0.5).unwrap_or(false);
                ui.label(format!("supported: {}", if supported { "yes" } else { "no" }));
                ui.label(format!("active/readback: {}", if enabled { "yes" } else { "no" }));
                ui.label("GPU rows appear under the wgpu.gpu group with a one-frame/readback delay.");
                ui.label("These are real GPU pass timestamps, not CPU encode times, but still per pass/bucket rather than WGSL line-by-line.");
            });

        if !report.notes.is_empty() {
            egui::CollapsingHeader::new("Notes")
                .default_open(false)
                .show(ui, |ui| {
                    for note in &report.notes {
                        ui.label(note);
                    }
                });
        }
    }
}
