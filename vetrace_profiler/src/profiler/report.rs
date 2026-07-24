fn profiler_notes() -> Vec<String> {
    vec![
        "wgpu GPU memory is estimated from Vetrace-owned resources, not OS VRAM usage".to_string(),
        "wgpu GPU timestamp rows are real pass-level QuerySet measurements when the adapter supports TIMESTAMP_QUERY; otherwise they stay disabled".to_string(),
        "detached profiler window overhead is recorded under profiler.self.* and is excluded from the rich UI's game timing view by default".to_string(),
    ]
}

fn profiler_panel_from_report(report: &ProfilerReport, top_n: usize) -> DebugTextOverlayPanel {
    let cpu = report.process_cpu_percent.map(|v| format!("{v:.1}%")).unwrap_or_else(|| "n/a".to_string());
    let ram = report.process_memory_bytes.map(format_bytes).unwrap_or_else(|| "n/a".to_string());
    let system_memory = match (report.system_memory_available_bytes, report.system_memory_total_bytes) {
        (Some(avail), Some(total)) => format!("{} free / {} total", format_bytes(avail), format_bytes(total)),
        _ => "system memory n/a".to_string(),
    };

    let mut lines = Vec::new();
    lines.push(format!("frame: {}", report.frame_index));
    lines.push(format!("frame time: {:.2} ms latest / {:.2} ms avg", report.latest_frame_ms, report.average_frame_ms));
    if !report.frame_history_ms.is_empty() {
        lines.push(format!("frame graph: {}", sparkline(&report.frame_history_ms, 40)));
    }
    lines.push(format!("fps: {:.1}", report.fps));
    lines.push(format!("cpu: {cpu}"));
    lines.push(format!("ram: {ram} ({system_memory})"));

    if !report.timings.is_empty() {
        lines.push("".to_string());
        lines.push("top timings:".to_string());
        let profiler_overhead_ms = profiler_overhead_ms(&report.timings);
        if profiler_overhead_ms > 0.001 {
            lines.push(format!("profiler.self overhead excluded from game view: {profiler_overhead_ms:.3} ms"));
        }
        for timing in report.timings.iter().filter(|timing| !is_profiler_overhead_name(&timing.name) && !is_profiler_polluted_parent_name(&timing.name)).take(top_n) {
            lines.push(format!(
                "{:34} {:>7.3} ms  x{:>2}  avg {:>6.3} ms  min/max {:>5.2}/{:>5.2}",
                timing.name,
                timing.total_ms,
                timing.calls,
                timing.average_ms,
                timing.rolling_min_ms,
                timing.rolling_max_ms,
            ));
        }
    }

    let interesting = report
        .counters
        .iter()
        .filter(|counter| counter.name.starts_with("wgpu.") || counter.name.starts_with("render.") || counter.name.starts_with("app."))
        .take(18)
        .collect::<Vec<_>>();
    if !interesting.is_empty() {
        lines.push("".to_string());
        lines.push("counters:".to_string());
        for counter in interesting {
            lines.push(format!("{:34} {}", counter.name, format_counter_value(counter)));
        }
    }

    DebugTextOverlayPanel {
        enabled: true,
        title: "Vetrace Profiler".to_string(),
        subtitle: "Runtime CPU/RAM/frame/WGPU timing view".to_string(),
        status: format!("{:.1} FPS | {:.2} ms avg", report.fps, report.average_frame_ms),
        lines,
        controls: vec![
            "Run with --profile to enable".to_string(),
            "Console output still prints every profiler interval".to_string(),
            "Sort/group/bar UI is available through vetrace_render/egui_overlay".to_string(),
            "GPU memory is an engine-owned estimate, not total OS VRAM".to_string(),
            "Real GPU pass timestamp rows appear as wgpu.gpu.* when the WGPU adapter supports TIMESTAMP_QUERY".to_string(),
        ],
    }
}

fn print_report(report: &ProfilerReport, top_n: usize) {
    let cpu = report.process_cpu_percent.map(|v| format!("{v:>5.1}%")).unwrap_or_else(|| "  n/a".to_string());
    let ram = report.process_memory_bytes.map(format_bytes).unwrap_or_else(|| "n/a".to_string());
    let available = match (report.system_memory_available_bytes, report.system_memory_total_bytes) {
        (Some(avail), Some(total)) => format!("{} free / {} total", format_bytes(avail), format_bytes(total)),
        _ => "system memory n/a".to_string(),
    };

    println!(
        "\n[vetrace_profiler] frame={} {:.2} ms avg {:.2} ms ({:.1} FPS) cpu={} ram={} ({})",
        report.frame_index,
        report.latest_frame_ms,
        report.average_frame_ms,
        report.fps,
        cpu,
        ram,
        available,
    );

    let profiler_overhead_ms = profiler_overhead_ms(&report.timings);
    if profiler_overhead_ms > 0.001 {
        println!("  profiler.self overhead excluded from main list: {profiler_overhead_ms:.3} ms");
    }

    for timing in report.timings.iter().filter(|timing| !is_profiler_overhead_name(&timing.name) && !is_profiler_polluted_parent_name(&timing.name)).take(top_n) {
        println!(
            "  {:42} latest {:>8.3} ms  calls {:>3}  avg/call {:>7.3} ms  hist avg/min/max {:>7.3}/{:>7.3}/{:>7.3} ms",
            timing.name,
            timing.total_ms,
            timing.calls,
            timing.average_ms,
            timing.rolling_average_ms,
            timing.rolling_min_ms,
            timing.rolling_max_ms,
        );
    }

    let profiler_timings = report.timings.iter().filter(|timing| is_profiler_overhead_name(&timing.name)).take(8).collect::<Vec<_>>();
    if !profiler_timings.is_empty() {
        println!("  profiler.self details:");
        for timing in profiler_timings {
            println!(
                "    {:38} latest {:>8.3} ms  calls {:>3}",
                timing.name,
                timing.total_ms,
                timing.calls,
            );
        }
    }

    let interesting = report
        .counters
        .iter()
        .filter(|counter| counter.name.starts_with("wgpu.") || counter.name.starts_with("render."))
        .take(18)
        .collect::<Vec<_>>();
    if !interesting.is_empty() {
        println!("  counters:");
        for counter in interesting {
            println!("    {:38} {}", counter.name, format_counter_value(counter));
        }
    }
}


fn is_profiler_overhead_name(name: &str) -> bool {
    name.starts_with("profiler.self.") || name.starts_with("plugin.profiler.") || name.starts_with("wgpu.profiler_window")
}

fn is_profiler_polluted_parent_name(name: &str) -> bool {
    matches!(name, "app.render" | "plugin.render.render" | "render.target_render")
}

fn profiler_overhead_ms(timings: &[ProfileTiming]) -> f32 {
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
        timings.iter().filter(|timing| is_profiler_overhead_name(&timing.name)).map(|timing| timing.total_ms).sum()
    }
}

fn format_counter_value(counter: &ProfileCounter) -> String {
    if counter.unit == "bytes" {
        format_bytes(counter.value.max(0.0) as u64)
    } else if counter.unit.is_empty() {
        format!("{:.0}", counter.value)
    } else {
        format!("{:.2} {}", counter.value, counter.unit)
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= KIB * KIB * KIB {
        format!("{:.2} GiB", bytes_f / KIB / KIB / KIB)
    } else if bytes_f >= KIB * KIB {
        format!("{:.2} MiB", bytes_f / KIB / KIB)
    } else if bytes_f >= KIB {
        format!("{:.2} KiB", bytes_f / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn trim_deque<T>(deque: &mut VecDeque<T>, max_len: usize) {
    while deque.len() > max_len {
        deque.pop_front();
    }
}

fn sparkline(values: &[f32], width: usize) -> String {
    if values.is_empty() || width == 0 { return String::new(); }
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let step = (values.len() as f32 / width as f32).max(1.0);
    let mut sampled = Vec::new();
    let mut index = 0.0f32;
    while (index as usize) < values.len() && sampled.len() < width {
        let start = index as usize;
        let end = ((index + step).ceil() as usize).min(values.len()).max(start + 1);
        let max_value = values[start..end].iter().copied().fold(0.0f32, f32::max);
        sampled.push(max_value);
        index += step;
    }
    let max_value = sampled.iter().copied().fold(0.0f32, f32::max).max(0.001);
    sampled
        .into_iter()
        .map(|value| {
            let t = (value / max_value).clamp(0.0, 1.0);
            let idx = (t * (BARS.len() as f32 - 1.0)).round() as usize;
            BARS[idx]
        })
        .collect()
}

fn min_duration(values: impl Iterator<Item = Duration>) -> Duration {
    values.min().unwrap_or(Duration::ZERO)
}

fn max_duration(values: impl Iterator<Item = Duration>) -> Duration {
    values.max().unwrap_or(Duration::ZERO)
}

fn average_duration(values: impl Iterator<Item = Duration>) -> Duration {
    let mut total = Duration::ZERO;
    let mut count = 0u32;
    for value in values {
        total += value;
        count = count.saturating_add(1);
    }
    if count == 0 { Duration::ZERO } else { total / count }
}

fn duration_ms(duration: Duration) -> f32 { duration.as_secs_f32() * 1000.0 }
