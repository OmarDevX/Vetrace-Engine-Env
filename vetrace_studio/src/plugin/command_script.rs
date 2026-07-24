use super::*;

impl StudioPlugin {
    pub(super) fn apply_script_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::OpenScript(path) => match self.scripts.open(path, None) {
                Ok(_) => self.status = "Opened script".to_string(),
                Err(error) => self.log(error),
            },
            StudioCommand::OpenScriptAt { path, line } => match self.scripts.open(path, Some(line)) {
                Ok(_) => self.status = format!("Opened script at line {line}"),
                Err(error) => self.log(error),
            },
            StudioCommand::SaveScript(index) => {
                self.save_script(index, false);
            }
            StudioCommand::SaveAndCloseScript(index) => {
                if self.save_script(index, true) {
                    let _ = self.scripts.with_state(|state| state.workspace.close(index, true));
                }
            }
            StudioCommand::SaveAllScripts => {
                self.save_all_scripts();
            }
            StudioCommand::RenameScript { index, project_path } => match self.scripts.rename_script(index, &project_path) {
                Ok(path) => {
                    self.status = format!("Renamed script to {}", path.display());
                    self.log(self.status.clone());
                    let _ = self.assets.refresh();
                    let project_root = self.project.root().to_path_buf();
                    match VetraceProject::load(&project_root) {
                        Ok(project) => self.project = project,
                        Err(error) => self.log(format!("Script renamed, but project reload failed: {error}")),
                    }
                }
                Err(error) => self.log(error),
            },
            StudioCommand::DeleteScript { index, discard } => match self.scripts.delete_script(index, discard) {
                Ok(path) => {
                    self.status = format!("Deleted script {}", path.display());
                    self.log(self.status.clone());
                    let _ = self.assets.refresh();
                }
                Err(error) => self.log(error),
            },
            StudioCommand::ResolveScriptExternal { path, resolution } => match self.scripts.resolve_external_change(&path, resolution) {
                Ok(()) => self.status = format!("Resolved external change for {}", path.display()),
                Err(error) => self.log(error),
            },
            StudioCommand::AssignLuaScript { entity, source } => {
                self.assign_lua_script(engine, entity, &source);
            }
            StudioCommand::CreateLuaScript { entity, project_path } => {
                self.create_and_assign_lua_script(engine, entity, &project_path);
            }
            _ => unreachable!("non-script command routed to script handler"),
        }
    }
}
