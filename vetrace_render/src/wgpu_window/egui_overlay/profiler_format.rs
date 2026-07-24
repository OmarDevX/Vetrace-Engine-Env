use super::*;

// Profiler colors, sparklines, and value formatting.

#[cfg(feature = "egui_render")]
impl WgpuRenderer {
    #[cfg(feature = "profiler")]
    pub(super) fn timing_color(value_ms: f32) -> egui::Color32 {
        if value_ms >= 8.0 {
            egui::Color32::from_rgb(255, 96, 96)
        } else if value_ms >= 4.0 {
            egui::Color32::from_rgb(255, 196, 64)
        } else {
            egui::Color32::from_rgb(140, 235, 140)
        }
    }

    #[cfg(feature = "profiler")]
    pub(super) fn budget_status(value_ms: f32, budget_ms: f32) -> &'static str {
        if value_ms > budget_ms { "over" } else if value_ms > budget_ms * 0.5 { "watch" } else { "ok" }
    }

    #[cfg(feature = "profiler")]
    pub(super) fn sparkline(values: &[f32], width: usize) -> String {
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

    #[cfg(feature = "profiler")]
    pub(super) fn format_counter_value(counter: &vetrace_profiler::ProfileCounter) -> String {
        if counter.unit == "bytes" {
            Self::format_bytes(counter.value.max(0.0) as u64)
        } else if counter.unit.is_empty() {
            format!("{:.0}", counter.value)
        } else {
            format!("{:.2} {}", counter.value, counter.unit)
        }
    }

    #[cfg(feature = "profiler")]
    pub(super) fn format_bytes(bytes: u64) -> String {
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
}
