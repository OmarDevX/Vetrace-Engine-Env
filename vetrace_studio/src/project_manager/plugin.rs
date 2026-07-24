use std::any::Any;
use std::error::Error;
use std::path::Path;

use vetrace_core::{Engine, Plugin};
use vetrace_project::VetraceProject;
use vetrace_render::{EguiToolRegistry, RenderSettings};

use super::{
    create_project, launch_studio_project, recent_projects, record_recent_project,
    remove_recent_path, ProjectManagerBridge, ProjectManagerCommand, ProjectManagerEguiTool,
    ProjectManagerSnapshot,
};

pub struct ProjectManagerPlugin {
    bridge: ProjectManagerBridge,
    recent: Vec<super::RecentProject>,
    status: String,
    busy: bool,
}

impl ProjectManagerPlugin {
    pub fn new() -> Self {
        Self {
            bridge: ProjectManagerBridge::default(),
            recent: recent_projects(),
            status: "Ready".to_string(),
            busy: false,
        }
    }

    fn refresh(&mut self) {
        self.recent = recent_projects();
        self.status = "Recent projects refreshed".to_string();
    }

    fn open_project(&mut self, engine: &mut Engine, path: &Path) {
        self.busy = true;
        self.status = format!("Opening {}…", path.display());
        let result = VetraceProject::discover(path)
            .or_else(|_| VetraceProject::load(path))
            .map_err(|error| error.to_string())
            .and_then(|project| {
                project
                    .validate_files()
                    .into_result()
                    .map_err(|error| error.to_string())?;
                record_recent_project(&project)?;
                launch_studio_project(project.root())?;
                Ok(project.root().to_path_buf())
            });
        match result {
            Ok(root) => {
                self.status = format!("Opened {}", root.display());
                engine.stop();
            }
            Err(error) => {
                self.status = error;
                self.busy = false;
            }
        }
    }

    fn apply_command(&mut self, engine: &mut Engine, command: ProjectManagerCommand) {
        match command {
            ProjectManagerCommand::Open(path) => self.open_project(engine, &path),
            ProjectManagerCommand::Create(request) => {
                self.busy = true;
                self.status = format!("Creating {}…", request.name);
                match create_project(&request) {
                    Ok(project) => {
                        if let Err(error) = record_recent_project(&project)
                            .and_then(|_| launch_studio_project(project.root()))
                        {
                            self.status = error;
                            self.busy = false;
                        } else {
                            self.status = format!("Created {}", project.root().display());
                            engine.stop();
                        }
                    }
                    Err(error) => {
                        self.status = error;
                        self.busy = false;
                    }
                }
            }
            ProjectManagerCommand::RemoveRecent(path) => match remove_recent_path(&path) {
                Ok(()) => self.refresh(),
                Err(error) => self.status = error,
            },
            ProjectManagerCommand::Refresh => self.refresh(),
            ProjectManagerCommand::Quit => engine.stop(),
        }
    }

    fn publish_snapshot(&self) {
        if let Ok(mut snapshot) = self.bridge.snapshot.lock() {
            *snapshot = ProjectManagerSnapshot {
                recent: self.recent.clone(),
                status: self.status.clone(),
                busy: self.busy,
            };
        }
    }
}

impl Default for ProjectManagerPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for ProjectManagerPlugin {
    fn name(&self) -> &'static str { "studio_project_manager" }

    fn dependencies(&self) -> Vec<&'static str> { vec!["render"] }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        if !engine.contains_resource::<EguiToolRegistry>() {
            engine.insert_resource(EguiToolRegistry::new());
        }
        if let Some(registry) = engine.get_resource::<EguiToolRegistry>().cloned() {
            registry.register(ProjectManagerEguiTool::new(self.bridge.clone()));
        }
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.title = "Vetrace Studio — Project Manager".to_string();
            settings.width = settings.width.max(1120);
            settings.height = settings.height.max(720);
            settings.cursor_grab = false;
            settings.cursor_visible = true;
            settings.clear_color = [0.025, 0.035, 0.055, 1.0];
        }
        self.publish_snapshot();
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, _dt: f32) -> Result<(), Box<dyn Error>> {
        for command in self.bridge.drain() {
            self.apply_command(engine, command);
            if !engine.is_running() {
                break;
            }
        }
        self.publish_snapshot();
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
