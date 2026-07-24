use super::*;

impl StudioPlugin {
    pub(super) fn assign_lua_script(
        &mut self,
        engine: &mut Engine,
        entity: vetrace_core::Entity,
        source: &std::path::Path,
    ) {
        match resolve_existing_script(&self.project, source) {
            Ok((project_path, resolved)) => {
                if let Err(error) = self.set_lua_script_path(engine, entity, project_path.as_str()) {
                    self.log(error);
                    return;
                }
                self.status = format!("Assigned {}", project_path.as_str());
                self.log(format!("Assigned Lua script {}", project_path.as_str()));
                self.mark_scene_changed("Assigned Lua script");
                if let Err(error) = self.scripts.open(&resolved, None) {
                    self.log(error);
                }
                if let Ok(status) = self.assets.refresh() {
                    self.log(status);
                }
            }
            Err(error) => self.log(error),
        }
    }

    pub(super) fn create_and_assign_lua_script(
        &mut self,
        engine: &mut Engine,
        entity: vetrace_core::Entity,
        requested: &str,
    ) {
        let entity_name = engine
            .actor(entity)
            .and_then(|actor| actor.name(engine).map(str::to_owned))
            .unwrap_or_else(|| "Entity".to_string());
        match create_lua_script(&self.project, requested, &entity_name) {
            Ok((project_path, resolved)) => {
                if let Err(error) = self.set_lua_script_path(engine, entity, project_path.as_str()) {
                    let _ = std::fs::remove_file(&resolved);
                    self.log(error);
                    return;
                }
                self.status = format!("Created {}", project_path.as_str());
                self.log(format!("Created and assigned Lua script {}", project_path.as_str()));
                self.mark_scene_changed("Created Lua script");
                if let Err(error) = self.scripts.open(&resolved, None) {
                    self.log(error);
                }
                if let Ok(status) = self.assets.refresh() {
                    self.log(status);
                }
            }
            Err(error) => self.log(error),
        }
    }

    fn set_lua_script_path(
        &mut self,
        engine: &mut Engine,
        entity: vetrace_core::Entity,
        project_path: &str,
    ) -> Result<(), String> {
        let actor = engine
            .actor(entity)
            .ok_or_else(|| "selected entity no longer exists".to_string())?;
        engine
            .set_registered_component_field(
                actor,
                LUA_SCRIPT_COMPONENT_ID,
                &FieldPath::root().field(LUA_SCRIPT_FIELD),
                DynamicValue::String(project_path.to_string()),
            )
            .map_err(|error| error.to_string())
    }

    pub(super) fn save_all_scripts(&mut self) -> bool {
        match self.scripts.save_all() {
            Ok(results) => {
                if results.is_empty() { return true; }
                let errors = results.iter().map(|result| result.error_count).sum::<usize>();
                self.status = if errors == 0 {
                    format!("Saved {} script(s)", results.len())
                } else {
                    format!("Saved {} script(s) with {errors} syntax error(s)", results.len())
                };
                for result in results {
                    self.log(format!("Saved {}", result.path.display()));
                }
                if let Ok(status) = self.assets.refresh() { self.log(status); }
                if !self.dirty { let _ = self.recovery.clear(); }
                true
            }
            Err(error) => {
                self.log(error);
                false
            }
        }
    }

    pub(super) fn save_script(&mut self, index: usize, closing: bool) -> bool {
        match self.scripts.save(index) {
            Ok(result) => {
                if result.error_count == 0 {
                    self.status = if self.player.is_running() {
                        "Script saved; live game will hot-reload it".to_string()
                    } else {
                        "Script saved".to_string()
                    };
                    self.log(format!("Saved {}", result.path.display()));
                } else {
                    self.status = format!(
                        "Script saved with {} syntax error(s); the running valid version was kept",
                        result.error_count,
                    );
                    self.log(format!(
                        "Saved {} with {} syntax error(s)",
                        result.path.display(), result.error_count,
                    ));
                }
                if let Ok(status) = self.assets.refresh() { self.log(status); }
                let _ = closing;
                true
            }
            Err(error) => {
                self.log(error);
                false
            }
        }
    }
}
