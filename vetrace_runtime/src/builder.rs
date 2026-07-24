use vetrace_core::{AppBuilder, FixedTime, Plugin};
use vetrace_project::VetraceProject;

use crate::{
    app::RuntimeApp,
    plugins::install_standard_plugins,
    settings::install_project_resources,
    RuntimeCapabilities, RuntimeConfig, RuntimeMode, RuntimeProject,
    RuntimeResult, RuntimeStatus, VetraceRuntime,
};

pub struct VetraceRuntimeBuilder {
    project: VetraceProject,
    config: RuntimeConfig,
    extra_plugins: Vec<Box<dyn Plugin>>,
}

impl VetraceRuntimeBuilder {
    pub fn new(project: VetraceProject) -> Self {
        Self { project, config: RuntimeConfig::default(), extra_plugins: Vec::new() }
    }

    pub fn mode(mut self, mode: RuntimeMode) -> Self {
        self.config.mode = mode;
        self
    }

    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn validate_project_files(mut self, validate: bool) -> Self {
        self.config.validate_project_files = validate;
        self
    }

    pub fn load_scene_assets(mut self, load: bool) -> Self {
        self.config.load_scene_assets = load;
        self
    }

    pub fn start_paused(mut self, paused: bool) -> Self {
        self.config.start_paused = paused;
        self
    }

    pub fn stop_on_window_close(mut self, stop: bool) -> Self {
        self.config.stop_on_window_close = stop;
        self
    }

    pub fn run_project_scripts(mut self, run: bool) -> Self {
        self.config.run_project_scripts = run;
        self
    }

    pub fn add_plugin<P: Plugin + 'static>(mut self, plugin: P) -> Self {
        self.extra_plugins.push(Box::new(plugin));
        self
    }

    pub fn add_boxed_plugin(mut self, plugin: Box<dyn Plugin>) -> Self {
        self.extra_plugins.push(plugin);
        self
    }

    pub fn build(self) -> RuntimeResult<VetraceRuntime> {
        self.project.validate_manifest().into_result()?;
        if self.config.validate_project_files {
            self.project.validate_files().into_result()?;
        }

        let mut engine = vetrace_core::Engine::new();
        install_project_resources(&mut engine, &self.project, &self.config);
        engine.insert_resource(RuntimeProject(self.project.clone()));
        engine.insert_resource(vetrace_scripting_lua::LuaProjectContext::new(self.project.clone()));
        engine.insert_resource(RuntimeCapabilities::compiled());
        engine.insert_resource(RuntimeStatus::new(self.config.mode));
        if !engine.contains_resource::<FixedTime>() {
            engine.insert_resource(FixedTime::default());
        }

        let app_builder = AppBuilder::with_engine(engine);
        let app_builder = install_standard_plugins(
            app_builder,
            &self.project,
            &self.config,
            self.extra_plugins,
        )?;
        let app = RuntimeApp::new(self.project.clone(), self.config.clone());
        Ok(VetraceRuntime::from_runner(
            self.project,
            self.config,
            app_builder.build(app),
        ))
    }
}

pub trait VetraceProjectRuntimeExt {
    fn runtime_builder(self) -> VetraceRuntimeBuilder;
}

impl VetraceProjectRuntimeExt for VetraceProject {
    fn runtime_builder(self) -> VetraceRuntimeBuilder {
        VetraceRuntimeBuilder::new(self)
    }
}
