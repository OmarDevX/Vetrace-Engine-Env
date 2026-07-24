use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};

use vetrace_core::{App, Engine, InputState};
use vetrace_project::{VetraceProject, PROJECT_MANIFEST_FILE};
use vetrace_scripting_lua::shutdown_autoload_scripts;

use crate::{
    scene_loader::{load_main_scene, unload_active_scene},
    scripting::load_autoload_scripts,
    settings::{apply_post_plugin_settings, apply_project_render_settings},
    ActiveRuntimeScene, RuntimeConfig, RuntimeError, RuntimeMode, RuntimeProject, RuntimeResult,
};

pub(crate) struct RuntimeApp {
    project: VetraceProject,
    config: RuntimeConfig,
    project_settings_fingerprint: Option<u64>,
    project_settings_poll_seconds: f32,
}

impl RuntimeApp {
    pub fn new(project: VetraceProject, config: RuntimeConfig) -> Self {
        let project_settings_fingerprint = manifest_fingerprint(&project);
        Self {
            project,
            config,
            project_settings_fingerprint,
            project_settings_poll_seconds: 0.0,
        }
    }

    pub fn reload_scene(&mut self, engine: &mut Engine) -> RuntimeResult<()> {
        if engine.contains_resource::<ActiveRuntimeScene>() {
            unload_active_scene(engine)?;
        }
        let scene = load_main_scene(engine, &self.project, &self.config)?;
        engine.insert_resource(scene);
        Ok(())
    }

    fn refresh_editor_project_settings(&mut self, engine: &mut Engine, dt: f32) {
        if self.config.mode != RuntimeMode::EditorPreview {
            return;
        }

        self.project_settings_poll_seconds += dt.max(0.0).min(0.1);
        if self.project_settings_poll_seconds < 0.25 {
            return;
        }
        self.project_settings_poll_seconds = 0.0;

        let fingerprint = manifest_fingerprint(&self.project);
        if fingerprint.is_none() || fingerprint == self.project_settings_fingerprint {
            return;
        }
        self.project_settings_fingerprint = fingerprint;

        let root = self.project.root().to_path_buf();
        match VetraceProject::load(&root) {
            Ok(project) => {
                apply_project_render_settings(engine, &project, RuntimeMode::EditorPreview);
                engine.insert_resource(RuntimeProject(project.clone()));
                engine.insert_resource(vetrace_scripting_lua::LuaProjectContext::new(
                    project.clone(),
                ));
                self.project = project;
                eprintln!(
                    "vetrace-runtime: refreshed saved project rendering settings in editor preview"
                );
            }
            Err(error) => eprintln!(
                "vetrace-runtime: project settings changed, but the saved manifest could not be reloaded: {error}"
            ),
        }
    }
}

impl App for RuntimeApp {
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        apply_post_plugin_settings(engine, &self.project);
        if self.config.run_project_scripts {
            load_autoload_scripts(engine, &self.project).map_err(box_runtime_error)?;
        }
        let scene = load_main_scene(engine, &self.project, &self.config).map_err(box_runtime_error)?;
        engine.insert_resource(scene);
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.refresh_editor_project_settings(engine, dt);

        let should_stop = self.config.stop_on_window_close
            && engine
                .get_resource::<InputState>()
                .map(InputState::quit_requested)
                .unwrap_or(false);
        if should_stop {
            engine.stop();
        }
    }

    fn shutdown(&mut self, engine: &mut Engine) {
        if engine.contains_resource::<ActiveRuntimeScene>() {
            let _ = unload_active_scene(engine);
        }
        if self.config.run_project_scripts && self.project.manifest().features.scripting {
            shutdown_autoload_scripts(engine);
        }
    }
}

fn manifest_fingerprint(project: &VetraceProject) -> Option<u64> {
    let bytes = std::fs::read(project.root().join(PROJECT_MANIFEST_FILE)).ok()?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Some(hasher.finish())
}

fn box_runtime_error(error: RuntimeError) -> Box<dyn Error> { Box::new(error) }
