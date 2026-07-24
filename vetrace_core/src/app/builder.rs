use std::error::Error;

use crate::{Engine, Stage};

use super::{App, AppRunner, Plugin, PluginManager};

pub struct AppBuilder {
    engine: Engine,
    plugin_manager: PluginManager,
}

impl AppBuilder {
    pub fn new() -> Self { Self { engine: Engine::new(), plugin_manager: PluginManager::new() } }
    pub fn with_engine(engine: Engine) -> Self { Self { engine, plugin_manager: PluginManager::new() } }

    pub fn add_plugin<P: Plugin + 'static>(mut self, plugin: P) -> Self {
        self.plugin_manager.add_plugin(plugin);
        self
    }

    pub fn add_boxed_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
        self.plugin_manager.add_boxed_plugin(plugin);
        self
    }

    pub fn add_system(
        mut self,
        stage: Stage,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) -> Self {
        self.engine.add_system(stage, name, system);
        self
    }

    pub fn add_system_before(
        mut self,
        stage: Stage,
        before: &str,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) -> Self {
        self.engine.add_system_before(stage, before, name, system);
        self
    }

    pub fn add_system_after(
        mut self,
        stage: Stage,
        after: &str,
        name: impl Into<String>,
        system: impl FnMut(&mut Engine, f32) + 'static,
    ) -> Self {
        self.engine.add_system_after(stage, after, name, system);
        self
    }

    pub fn insert_resource<T: 'static>(mut self, value: T) -> Self {
        self.engine.insert_resource(value);
        self
    }

    pub fn build<A: App>(self, app: A) -> AppRunner<A> {
        AppRunner::new(
            self.engine,
            self.plugin_manager,
            app,
        )
    }

    pub fn run<A: App>(self, app: A) -> Result<(), Box<dyn Error>> {
        self.run_frames(app, 1, 0.0)
    }

    pub fn run_frames<A: App>(self, app: A, frames: usize, dt: f32) -> Result<(), Box<dyn Error>> {
        let mut runner = self.build(app);
        runner.run_frames(frames, dt)
    }

    pub fn run_until_stopped<A: App>(
        self,
        app: A,
        max_frames: Option<usize>,
        dt: f32,
    ) -> Result<(), Box<dyn Error>> {
        let mut runner = self.build(app);
        runner.run_until_stopped(max_frames, dt)
    }
}

impl Default for AppBuilder {
    fn default() -> Self { Self::new() }
}
