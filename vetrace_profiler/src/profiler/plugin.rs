pub struct ProfilerPlugin {
    config: ProfilerConfig,
    shared: Arc<Mutex<ProfilerInner>>,
    last_print: Instant,
}

impl ProfilerPlugin {
    pub fn new(config: ProfilerConfig) -> Self {
        let shared = Arc::new(Mutex::new(ProfilerInner::new(config.clone())));
        Self { config, shared, last_print: Instant::now() }
    }

    pub fn with_interval(interval: Duration) -> Self {
        Self::new(ProfilerConfig { print_interval: interval, ..ProfilerConfig::default() })
    }

    pub fn disabled() -> Self {
        Self::new(ProfilerConfig { enabled: false, ..ProfilerConfig::default() })
    }
}

impl Default for ProfilerPlugin {
    fn default() -> Self { Self::new(ProfilerConfig::default()) }
}

impl Plugin for ProfilerPlugin {
    fn name(&self) -> &'static str { "profiler" }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        set_global_profiler(&self.shared);
        engine.insert_resource::<Box<dyn ProfilerBackend>>(Box::new(ProfilerBackendBridge { shared: self.shared.clone() }));
        engine.insert_resource(ProfilerReport::default());
        engine.insert_resource(ProfilerUiSettings { mode: self.config.ui_mode });
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        if !self.config.enabled { return Ok(()); }
        let report = {
            let mut inner = self.shared.lock().expect("profiler lock poisoned");
            if self.config.sample_process {
                inner.sample_process();
            }
            inner.report.clone()
        };
        engine.insert_resource(report.clone());
        engine.insert_resource(ProfilerUiSettings { mode: self.config.ui_mode });
        if matches!(self.config.ui_mode, ProfilerUiMode::Overlay | ProfilerUiMode::Both) {
            engine.insert_resource(profiler_panel_from_report(&report, self.config.top_timing_count));
        } else {
            engine.remove_resource::<DebugTextOverlayPanel>();
        }

        if self.config.print_to_stdout && self.last_print.elapsed() >= self.config.print_interval {
            self.last_print = Instant::now();
            print_report(&report, self.config.top_timing_count);
        }
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

struct ProfilerBackendBridge {
    shared: Arc<Mutex<ProfilerInner>>,
}

impl ProfilerBackend for ProfilerBackendBridge {
    fn begin_frame(&mut self) {
        if let Ok(mut inner) = self.shared.lock() {
            inner.begin_frame();
        }
    }

    fn end_frame(&mut self) {
        if let Ok(mut inner) = self.shared.lock() {
            inner.end_frame();
        }
    }

    fn record_timing(&mut self, name: &str, duration: Duration) {
        if let Ok(mut inner) = self.shared.lock() {
            inner.record_timing(name, duration);
        }
    }

    fn record_counter(&mut self, name: &str, value: f64, unit: &'static str) {
        if let Ok(mut inner) = self.shared.lock() {
            inner.record_counter(name, value, unit);
        }
    }
}

