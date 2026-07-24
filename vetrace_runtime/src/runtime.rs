use std::error::Error;
use std::time::{Duration, Instant};

use vetrace_core::{AppRunner, Engine};
use vetrace_project::VetraceProject;

use crate::{
    app::RuntimeApp, RuntimeConfig, RuntimeError, RuntimeResult, RuntimeState,
    RuntimeStatus, VetraceRuntimeBuilder,
};

pub struct VetraceRuntime {
    project: VetraceProject,
    config: RuntimeConfig,
    runner: AppRunner<RuntimeApp>,
    state: RuntimeState,
}

impl VetraceRuntime {
    pub fn builder(project: VetraceProject) -> VetraceRuntimeBuilder {
        VetraceRuntimeBuilder::new(project)
    }

    pub fn load(project: VetraceProject, mode: crate::RuntimeMode) -> RuntimeResult<Self> {
        Self::builder(project).mode(mode).build()
    }

    pub(crate) fn from_runner(
        project: VetraceProject,
        config: RuntimeConfig,
        runner: AppRunner<RuntimeApp>,
    ) -> Self {
        Self { project, config, runner, state: RuntimeState::Created }
    }

    pub fn start(&mut self) -> RuntimeResult<()> {
        if self.state != RuntimeState::Created {
            return Err(RuntimeError::InvalidState { operation: "start", state: self.state });
        }

        self.set_state(RuntimeState::Starting);
        if let Err(error) = self.runner.initialize() {
            self.set_state(RuntimeState::Failed);
            return Err(runtime_error_from_box(error));
        }
        self.set_state(if self.config.start_paused {
            RuntimeState::Paused
        } else {
            RuntimeState::Running
        });
        Ok(())
    }

    pub fn update(&mut self, dt: f32) -> RuntimeResult<()> {
        validate_delta(dt)?;
        if self.state == RuntimeState::Created {
            self.start()?;
        }
        match self.state {
            RuntimeState::Running => self.run_frame(dt)?,
            RuntimeState::Paused => self.run_frame(0.0)?,
            state => return Err(RuntimeError::InvalidState { operation: "update", state }),
        }
        if !self.runner.engine().is_running() {
            self.finish_stop();
        }
        Ok(())
    }

    pub fn run_frames(&mut self, frames: usize, dt: f32) -> RuntimeResult<()> {
        validate_delta(dt)?;
        if self.state == RuntimeState::Created {
            self.start()?;
        }
        for _ in 0..frames {
            if !matches!(self.state, RuntimeState::Running | RuntimeState::Paused) { break; }
            self.update(dt)?;
        }
        Ok(())
    }

    pub fn run_until_stopped(&mut self, max_frames: Option<usize>, dt: f32) -> RuntimeResult<()> {
        validate_delta(dt)?;
        if self.state == RuntimeState::Created {
            self.start()?;
        }
        let target_frame = (dt > 0.0).then(|| Duration::from_secs_f32(dt));
        let mut frames = 0usize;
        while matches!(self.state, RuntimeState::Running | RuntimeState::Paused) {
            if max_frames.is_some_and(|max| frames >= max) { break; }
            let frame_start = Instant::now();
            self.update(dt)?;
            frames = frames.saturating_add(1);
            if let Some(target) = target_frame {
                let elapsed = frame_start.elapsed();
                if elapsed < target {
                    std::thread::sleep(target - elapsed);
                }
            }
        }
        Ok(())
    }

    pub fn pause(&mut self) -> RuntimeResult<()> {
        if self.state != RuntimeState::Running {
            return Err(RuntimeError::InvalidState { operation: "pause", state: self.state });
        }
        self.set_state(RuntimeState::Paused);
        Ok(())
    }

    pub fn resume(&mut self) -> RuntimeResult<()> {
        if self.state != RuntimeState::Paused {
            return Err(RuntimeError::InvalidState { operation: "resume", state: self.state });
        }
        self.set_state(RuntimeState::Running);
        Ok(())
    }

    pub fn reload_scene(&mut self) -> RuntimeResult<()> {
        if !matches!(self.state, RuntimeState::Running | RuntimeState::Paused) {
            return Err(RuntimeError::InvalidState { operation: "reload the scene", state: self.state });
        }
        let (app, engine) = self.runner.app_engine_mut();
        app.reload_scene(engine)
    }

    pub fn stop(&mut self) -> RuntimeResult<()> {
        match self.state {
            RuntimeState::Created => {
                self.runner.engine_mut().stop();
                self.finish_stop();
            }
            RuntimeState::Starting | RuntimeState::Running | RuntimeState::Paused | RuntimeState::Failed => {
                self.set_state(RuntimeState::Stopping);
                self.runner.engine_mut().stop();
                self.finish_stop();
            }
            RuntimeState::Stopping | RuntimeState::Stopped => {}
        }
        Ok(())
    }

    pub fn state(&self) -> RuntimeState { self.state }
    pub fn project(&self) -> &VetraceProject { &self.project }
    pub fn config(&self) -> &RuntimeConfig { &self.config }
    pub fn engine(&self) -> &Engine { self.runner.engine() }
    pub fn engine_mut(&mut self) -> &mut Engine { self.runner.engine_mut() }
    pub fn frame_count(&self) -> u64 { self.runner.frame_count() }
    pub fn status(&self) -> Option<&RuntimeStatus> { self.runner.engine().get_resource::<RuntimeStatus>() }
    pub fn active_scene(&self) -> Option<&crate::ActiveRuntimeScene> {
        self.runner.engine().get_resource::<crate::ActiveRuntimeScene>()
    }
    pub fn diagnostics(&self) -> Option<&crate::RuntimeDiagnostics> {
        self.runner.engine().get_resource::<crate::RuntimeDiagnostics>()
    }

    fn run_frame(&mut self, dt: f32) -> RuntimeResult<()> {
        if let Err(error) = self.runner.run_frame(dt) {
            self.set_state(RuntimeState::Failed);
            return Err(runtime_error_from_box(error));
        }
        if let Some(status) = self.runner.engine_mut().get_resource_mut::<RuntimeStatus>() {
            status.frame = status.frame.saturating_add(1);
            status.delta_seconds = dt;
            status.elapsed = status.elapsed.saturating_add(Duration::from_secs_f32(dt));
        }
        Ok(())
    }

    fn finish_stop(&mut self) {
        self.runner.shutdown();
        self.set_state(RuntimeState::Stopped);
    }

    fn set_state(&mut self, state: RuntimeState) {
        self.state = state;
        if let Some(status) = self.runner.engine_mut().get_resource_mut::<RuntimeStatus>() {
            status.state = state;
        }
    }
}

impl Drop for VetraceRuntime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn runtime_error_from_box(error: Box<dyn Error>) -> RuntimeError {
    match error.downcast::<RuntimeError>() {
        Ok(error) => *error,
        Err(error) => RuntimeError::Plugin(error.to_string()),
    }
}

fn validate_delta(dt: f32) -> RuntimeResult<()> {
    if dt.is_finite() && dt >= 0.0 {
        Ok(())
    } else {
        Err(RuntimeError::InvalidDelta(dt))
    }
}
