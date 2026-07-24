use super::*;

impl StudioPlugin {
    pub(super) fn apply_project_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::SaveProjectSettings(manifest) => {
                let previous = self.project.manifest().clone();
                *self.project.manifest_mut() = manifest;
                match self.project.save() {
                    Ok(()) => {
                        self.project_revision = self.project_revision.saturating_add(1);
                        self.status = "Project settings saved".to_owned();
                        self.log("Saved project.vetrace.toml");
                        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
                            settings.title = format!("Vetrace Studio — {}", self.project.manifest().project.name);
                        }
                    }
                    Err(error) => {
                        *self.project.manifest_mut() = previous;
                        self.log(format!("Project settings were not saved: {error}"));
                    }
                }
            }
            StudioCommand::OpenProjectManager => {
                if self.dirty || self.scripts.has_dirty_documents() {
                    self.status = "Project manager blocked: authored files have unsaved changes".to_string();
                } else {
                    self.open_project_manager(engine);
                }
            }
            StudioCommand::OpenProjectManagerDiscard => self.open_project_manager(engine),
            StudioCommand::SaveAndOpenProjectManager => {
                if self.save_scene(engine) && self.save_all_scripts() {
                    self.open_project_manager(engine);
                }
            }
            StudioCommand::Quit => {
                if self.dirty || self.scripts.has_dirty_documents() {
                    self.status = "Quit blocked: authored files have unsaved changes".to_string();
                    self.log("Save the scene before quitting, or use the discard confirmation");
                } else {
                    engine.stop();
                }
            }
            StudioCommand::QuitDiscard => engine.stop(),
            StudioCommand::SaveAndQuit => {
                if self.save_scene(engine) && self.save_all_scripts() {
                    engine.stop();
                }
            }
            _ => unreachable!("non-project command routed to project handler"),
        }
    }
}
