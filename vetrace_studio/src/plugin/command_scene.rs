use super::*;

impl StudioPlugin {
    pub(super) fn apply_scene_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::Undo => self.undo(engine),
            StudioCommand::Redo => self.redo(engine),
            StudioCommand::SaveScene => {
                self.save_scene(engine);
            }
            StudioCommand::NewScene(path) => {
                self.open_or_create_scene(engine, path, true);
            }
            StudioCommand::OpenScene(path) => {
                self.open_or_create_scene(engine, path, false);
            }
            StudioCommand::SaveSceneAs(path) => {
                let result = self.project.paths().to_project_path(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|path| save_active_scene_as(engine, &self.project, path));
                match result {
                    Ok(document) => {
                        self.reset_scene_history(engine);
                        self.status = format!("Saved scene as {} objects", document.object_count());
                        self.log(format!("Saved scene as {}", path.display()));
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::RecoverSession => {
                let Some(bundle) = self.recovery.take() else {
                    self.status = "No recovery session is available".to_owned();
                    return;
                };
                match restore_scene_document(
                    engine,
                    &self.project,
                    bundle.scene_path.clone(),
                    bundle.scene,
                ) {
                    Ok(()) => {
                        self.reset_scene_history(engine);
                        self.mark_scene_changed("Recovered unsaved scene");
                        for message in self.scripts.restore_recovery_scripts(&self.project, &bundle.scripts) {
                            self.log(message);
                        }
                        self.status = "Recovered autosaved scene and scripts".to_owned();
                        self.log("Recovered the previous Studio session");
                    }
                    Err(error) => self.log(format!("Recovery failed: {error}")),
                }
                let _ = self.recovery.clear();
            }
            StudioCommand::DiscardRecovery => {
                match self.recovery.clear() {
                    Ok(()) => self.status = "Discarded recovery session".to_owned(),
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::SetCurrentSceneAsMain => {
                let path = active_scene_project_path(engine, &self.project);
                self.project.manifest_mut().runtime.main_scene = path.clone();
                match self.project.save() {
                    Ok(()) => {
                        self.status = "Updated project main scene".to_owned();
                        self.log(format!("Main scene set to {path}"));
                    }
                    Err(error) => self.log(error.to_string()),
                }
            }
            StudioCommand::ReloadScene => {
                if self.dirty {
                    self.status = "Reload blocked: scene has unsaved changes".to_string();
                    self.log("Save the scene before reloading, or use the discard confirmation");
                } else {
                    self.reload_scene(engine);
                }
            }
            StudioCommand::ReloadSceneDiscard => self.reload_scene(engine),
            StudioCommand::SaveAndReload => {
                if self.save_scene(engine) && self.save_all_scripts() {
                    self.reload_scene(engine);
                }
            }
            _ => unreachable!("non-scene command routed to scene handler"),
        }
    }
}
