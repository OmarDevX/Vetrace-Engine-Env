#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfilerUiMode {
    /// No profiler UI. Console printing and report resources can still be active.
    None,
    /// Draw the profiler inside the main game window. Useful as a fallback.
    Overlay,
    /// Draw the profiler in a second native OS window. Recommended for gameplay.
    Detached,
    /// Draw both the detached native window and the in-game overlay.
    Both,
}

impl Default for ProfilerUiMode {
    fn default() -> Self { Self::Detached }
}

#[derive(Clone, Debug)]
pub struct ProfilerUiSettings {
    pub mode: ProfilerUiMode,
}

impl Default for ProfilerUiSettings {
    fn default() -> Self { Self { mode: ProfilerUiMode::Detached } }
}

#[derive(Clone, Debug)]
pub struct ProfilerConfig {
    pub enabled: bool,
    pub print_to_stdout: bool,
    pub print_interval: Duration,
    pub history_frames: usize,
    pub top_timing_count: usize,
    pub sample_process: bool,
    /// Profiler UI placement. Detached is the default so gameplay mouse capture
    /// does not block interacting with the profiler.
    pub ui_mode: ProfilerUiMode,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            print_to_stdout: true,
            print_interval: Duration::from_secs(2),
            history_frames: 180,
            top_timing_count: 14,
            sample_process: true,
            ui_mode: ProfilerUiMode::Detached,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProfilerReport {
    pub frame_index: u64,
    pub latest_frame_ms: f32,
    pub average_frame_ms: f32,
    pub fps: f32,
    pub process_cpu_percent: Option<f32>,
    pub process_memory_bytes: Option<u64>,
    pub system_memory_total_bytes: Option<u64>,
    pub system_memory_available_bytes: Option<u64>,
    pub frame_history_ms: Vec<f32>,
    pub timings: Vec<ProfileTiming>,
    pub counters: Vec<ProfileCounter>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ProfileTiming {
    pub name: String,
    /// Number of times this scope was recorded in the latest frame.
    pub calls: u32,
    /// Total time spent in this scope in the latest frame.
    pub total_ms: f32,
    /// Average time per call in the latest frame.
    pub average_ms: f32,
    /// Slowest single call in the latest frame.
    pub max_ms: f32,
    /// Rolling average of the per-frame total over the configured history window.
    pub rolling_average_ms: f32,
    /// Rolling minimum of the per-frame total over the configured history window.
    pub rolling_min_ms: f32,
    /// Rolling maximum of the per-frame total over the configured history window.
    pub rolling_max_ms: f32,
}

#[derive(Clone, Debug, Default)]
pub struct ProfileCounter {
    pub name: String,
    pub value: f64,
    pub unit: &'static str,
}

