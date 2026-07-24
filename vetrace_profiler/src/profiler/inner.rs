#[derive(Default)]
struct TimingAccum {
    calls: u32,
    total: Duration,
    max: Duration,
}

impl TimingAccum {
    fn add(&mut self, duration: Duration) {
        self.calls = self.calls.saturating_add(1);
        self.total += duration;
        self.max = self.max.max(duration);
    }
}

struct ProfilerInner {
    config: ProfilerConfig,
    frame_start: Option<Instant>,
    frame_times: VecDeque<Duration>,
    current_timings: HashMap<String, TimingAccum>,
    rolling_totals: HashMap<String, VecDeque<Duration>>,
    counters: HashMap<String, ProfileCounter>,
    process_sampler: ProcessSampler,
    report: ProfilerReport,
}

impl ProfilerInner {
    fn new(config: ProfilerConfig) -> Self {
        Self {
            config,
            frame_start: None,
            frame_times: VecDeque::new(),
            current_timings: HashMap::new(),
            rolling_totals: HashMap::new(),
            counters: HashMap::new(),
            process_sampler: ProcessSampler::default(),
            report: ProfilerReport {
                notes: profiler_notes(),
                ..ProfilerReport::default()
            },
        }
    }

    fn begin_frame(&mut self) {
        if !self.config.enabled { return; }
        self.frame_start = Some(Instant::now());
        self.current_timings.clear();
    }

    fn end_frame(&mut self) {
        if !self.config.enabled { return; }
        let frame_time = self.frame_start.take().map(|start| start.elapsed()).unwrap_or_default();
        self.frame_times.push_back(frame_time);
        trim_deque(&mut self.frame_times, self.config.history_frames.max(1));

        let mut timings = Vec::with_capacity(self.current_timings.len());
        for (name, accum) in &self.current_timings {
            let rolling = self.rolling_totals.entry(name.clone()).or_default();
            rolling.push_back(accum.total);
            trim_deque(rolling, self.config.history_frames.max(1));
            let rolling_average = average_duration(rolling.iter().copied());
            timings.push(ProfileTiming {
                name: name.clone(),
                calls: accum.calls,
                total_ms: duration_ms(accum.total),
                average_ms: if accum.calls > 0 { duration_ms(accum.total) / accum.calls as f32 } else { 0.0 },
                max_ms: duration_ms(accum.max),
                rolling_average_ms: duration_ms(rolling_average),
                rolling_min_ms: duration_ms(min_duration(rolling.iter().copied())),
                rolling_max_ms: duration_ms(max_duration(rolling.iter().copied())),
            });
        }
        timings.sort_by(|a, b| b.total_ms.partial_cmp(&a.total_ms).unwrap_or(std::cmp::Ordering::Equal));

        let average_frame = average_duration(self.frame_times.iter().copied());
        let mut counters = self.counters.values().cloned().collect::<Vec<_>>();
        counters.sort_by(|a, b| a.name.cmp(&b.name));

        self.report.frame_index = self.report.frame_index.saturating_add(1);
        self.report.latest_frame_ms = duration_ms(frame_time);
        self.report.average_frame_ms = duration_ms(average_frame);
        self.report.fps = if average_frame.as_secs_f32() > 0.0 { 1.0 / average_frame.as_secs_f32() } else { 0.0 };
        self.report.frame_history_ms = self.frame_times.iter().copied().map(duration_ms).collect();
        self.report.timings = timings;
        self.report.counters = counters;
        self.report.notes = profiler_notes();
    }

    fn record_timing(&mut self, name: &str, duration: Duration) {
        if !self.config.enabled { return; }
        self.current_timings.entry(name.to_string()).or_default().add(duration);
    }

    fn record_counter(&mut self, name: &str, value: f64, unit: &'static str) {
        if !self.config.enabled { return; }
        self.counters.insert(name.to_string(), ProfileCounter { name: name.to_string(), value, unit });
    }

    fn sample_process(&mut self) {
        let sample = self.process_sampler.sample();
        self.report.process_cpu_percent = sample.cpu_percent;
        self.report.process_memory_bytes = sample.memory_bytes;
        self.report.system_memory_total_bytes = sample.system_memory_total_bytes;
        self.report.system_memory_available_bytes = sample.system_memory_available_bytes;
    }
}

pub struct ScopeTimer {
    name: &'static str,
    start: Instant,
}

impl ScopeTimer {
    pub fn new(name: &'static str) -> Self { Self { name, start: Instant::now() } }
}

impl Drop for ScopeTimer {
    fn drop(&mut self) {
        record_timing(self.name, self.start.elapsed());
    }
}

#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _vetrace_profile_scope = $crate::ScopeTimer::new($name);
    };
}

pub fn record_timing(name: &str, duration: Duration) {
    with_global(|inner| inner.record_timing(name, duration));
}

pub fn record_counter(name: &str, value: f64, unit: &'static str) {
    with_global(|inner| inner.record_counter(name, value, unit));
}

pub fn record_memory_bytes(name: &str, bytes: u64) {
    record_counter(name, bytes as f64, "bytes");
}

fn with_global(f: impl FnOnce(&mut ProfilerInner)) {
    let Some(lock) = GLOBAL_PROFILER.get() else { return; };
    let Ok(weak) = lock.lock() else { return; };
    let Some(shared) = weak.upgrade() else { return; };
    drop(weak);
    if let Ok(mut inner) = shared.lock() {
        f(&mut inner);
    }
}

fn set_global_profiler(shared: &Arc<Mutex<ProfilerInner>>) {
    let lock = GLOBAL_PROFILER.get_or_init(|| Mutex::new(Weak::new()));
    if let Ok(mut weak) = lock.lock() {
        *weak = Arc::downgrade(shared);
    }
}


