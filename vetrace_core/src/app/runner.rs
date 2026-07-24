use std::error::Error;
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::{Engine, Stage};

use super::frame_pacing::{pace_frame, validate_frame_delta};
use super::{App, PluginManager};

/// An initialized-or-initializable app instance whose engine can be advanced a
/// frame at a time. Product runtimes and editors use this instead of surrendering
/// control to `AppBuilder::run_until_stopped`.
pub struct AppRunner<A: App> {
    engine: Engine,
    plugin_manager: PluginManager,
    app: A,
    initialized: bool,
    shutdown: bool,
    frame_count: u64,
}

impl<A: App> AppRunner<A> {
    pub(super) fn new(engine: Engine, plugin_manager: PluginManager, app: A) -> Self {
        Self {
            engine,
            plugin_manager,
            app,
            initialized: false,
            shutdown: false,
            frame_count: 0,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn Error>> {
        if self.initialized { return Ok(()); }
        self.plugin_manager.initialize_plugins(&mut self.engine)?;
        let started = Instant::now();
        self.app.initialize(&mut self.engine)?;
        self.engine.profile_record_timing("app.setup", started.elapsed());
        self.engine.run_stage(Stage::Startup, 0.0);
        self.initialized = true;
        Ok(())
    }

    pub fn run_frame(&mut self, dt: f32) -> Result<(), Box<dyn Error>> {
        validate_frame_delta(dt)?;
        self.initialize()?;
        if self.shutdown || !self.engine.is_running() { return Ok(()); }

        self.engine.profile_begin_frame();
        self.engine.profile_record_counter("app.dt", dt as f64, "seconds");

        self.run_update_stage(Stage::PreUpdate, dt)?;

        // Preserve the engine's gameplay-before-subsystems behavior: the app
        // records input/intent first, then fixed simulation consumes it.
        let started = Instant::now();
        self.app.update(&mut self.engine, dt);
        self.engine.profile_record_timing("app.update", started.elapsed());
        self.run_update_stage(Stage::Update, dt)?;

        let (fixed_steps, fixed_dt) = self.engine.fixed_steps_for_frame(dt);
        for _ in 0..fixed_steps {
            self.run_update_stage(Stage::FixedUpdate, fixed_dt)?;
            self.run_update_stage(Stage::Physics, fixed_dt)?;
            self.run_update_stage(Stage::PostPhysics, fixed_dt)?;
        }

        self.run_update_stage(Stage::PostUpdate, dt)?;
        self.run_update_stage(Stage::RenderExtract, dt)?;

        let started = Instant::now();
        self.app.render(&mut self.engine);
        self.engine.profile_record_timing("app.render", started.elapsed());
        self.engine.run_stage(Stage::Render, dt);
        self.plugin_manager.render_stage(Stage::Render, &mut self.engine)?;
        self.run_update_stage(Stage::Cleanup, dt)?;

        self.engine.flush_commands();
        self.engine.profile_end_frame();
        self.frame_count = self.frame_count.saturating_add(1);
        Ok(())
    }

    pub fn run_frames(&mut self, frames: usize, dt: f32) -> Result<(), Box<dyn Error>> {
        validate_frame_delta(dt)?;
        self.initialize()?;
        for _ in 0..frames {
            if !self.engine.is_running() { break; }
            self.run_frame(dt)?;
        }
        if !self.engine.is_running() { self.shutdown(); }
        Ok(())
    }

    pub fn run_until_stopped(&mut self, max_frames: Option<usize>, dt: f32) -> Result<(), Box<dyn Error>> {
        validate_frame_delta(dt)?;
        self.initialize()?;
        let target_frame = (dt > 0.0).then(|| Duration::from_secs_f32(dt));
        let mut frame = 0usize;
        while self.engine.is_running() {
            if max_frames.map(|max| frame >= max).unwrap_or(false) { break; }
            let frame_start = Instant::now();
            self.run_frame(dt)?;
            frame += 1;
            if let Some(target_frame) = target_frame { pace_frame(frame_start, target_frame); }
        }
        if !self.engine.is_running() { self.shutdown(); }
        Ok(())
    }

    pub fn shutdown(&mut self) {
        if self.shutdown { return; }
        if self.initialized {
            self.app.shutdown(&mut self.engine);
            self.engine.flush_commands();
        }
        self.shutdown = true;
    }

    pub fn engine(&self) -> &Engine { &self.engine }
    pub fn engine_mut(&mut self) -> &mut Engine { &mut self.engine }
    pub fn app(&self) -> &A { &self.app }
    pub fn app_mut(&mut self) -> &mut A { &mut self.app }
    pub fn app_engine_mut(&mut self) -> (&mut A, &mut Engine) { (&mut self.app, &mut self.engine) }
    pub fn is_initialized(&self) -> bool { self.initialized }
    pub fn is_shutdown(&self) -> bool { self.shutdown }
    pub fn frame_count(&self) -> u64 { self.frame_count }

    fn run_update_stage(&mut self, stage: Stage, dt: f32) -> Result<(), Box<dyn Error>> {
        self.engine.run_stage(stage, dt);
        self.plugin_manager.update_stage(stage, &mut self.engine, dt)?;
        self.engine.flush_commands();
        Ok(())
    }
}

impl<A: App> Drop for AppRunner<A> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
