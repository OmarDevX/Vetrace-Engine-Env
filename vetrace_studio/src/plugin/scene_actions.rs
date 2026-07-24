use super::*;

impl StudioPlugin {
    pub(super) fn reset_scene_history(&mut self, engine: &mut Engine) {
        match capture_authored_scene(engine) {
            Ok(snapshot) => {
                self.saved_fingerprint = snapshot.fingerprint().to_vec();
                self.history.reset(snapshot);
                self.history_ready = true;
                self.history_pending = false;
                self.history_idle_seconds = 0.0;
                self.dirty = false;
                self.transform_signature = Some(authored_transform_signature(engine));
            }
            Err(error) => self.log(error),
        }
    }

    pub(super) fn open_or_create_scene(&mut self, engine: &mut Engine, absolute: std::path::PathBuf, create: bool) {
        let result = self.project.paths().to_project_path(&absolute)
            .map_err(|error| error.to_string())
            .and_then(|path| {
                if create { create_scene(engine, &self.project, path) }
                else { open_scene(engine, &self.project, path) }
            });
        match result {
            Ok(()) => {
                self.reset_scene_history(engine);
                self.status = if create { "Created scene".to_owned() } else { "Opened scene".to_owned() };
                self.log(format!("{} {}", if create { "Created" } else { "Opened" }, absolute.display()));
            }
            Err(error) => self.log(error),
        }
    }

    pub(super) fn save_scene(&mut self, engine: &mut Engine) -> bool {
        self.record_current_history(engine);
        match save_active_scene(engine, &self.project) {
            Ok(document) => {
                if let Some(snapshot) = self.history.current() {
                    self.saved_fingerprint = snapshot.fingerprint().to_vec();
                }
                self.dirty = false;
                self.history_pending = false;
                self.transform_signature = Some(authored_transform_signature(engine));
                self.status = format!("Saved {} objects", document.object_count());
                let path = active_scene_project_path(engine, &self.project);
                self.log(format!("Saved {}", path));
                if !self.scripts.has_dirty_documents() {
                    let _ = self.recovery.clear();
                }
                true
            }
            Err(error) => {
                self.log(error);
                false
            }
        }
    }

    pub(super) fn reload_scene(&mut self, engine: &mut Engine) {
        match reload_active_scene(engine, &self.project) {
            Ok(()) => match capture_authored_scene(engine) {
                Ok(snapshot) => {
                    self.saved_fingerprint = snapshot.fingerprint().to_vec();
                    self.history.reset(snapshot);
                    self.history_ready = true;
                    self.history_pending = false;
                    self.history_idle_seconds = 0.0;
                    self.dirty = false;
                    self.transform_signature = Some(authored_transform_signature(engine));
                    self.status = "Scene reloaded".to_string();
                    self.log("Reloaded the active scene from disk");
                }
                Err(error) => self.log(error),
            },
            Err(error) => self.log(error),
        }
    }

    pub(super) fn track_editor_changes(&mut self, engine: &Engine) {
        let signature = authored_transform_signature(engine);
        if let Some(previous) = self.transform_signature {
            if previous != signature {
                self.mark_scene_changed("Transform edit");
            }
        }
        self.transform_signature = Some(signature);
    }

    pub(super) fn refresh_snapshot(&mut self, engine: &Engine, player_running: bool) {
        let (assets, asset_diagnostics, asset_cache) = self.assets.snapshot();
        let mut snapshot = StudioSnapshot {
            project_name: self.project.manifest().project.name.clone(),
            project_root: self.project.root().to_path_buf(),
            scene_path: active_scene_project_path(engine, &self.project).to_string(),
            dirty: self.dirty,
            status: self.status.clone(),
            assets,
            asset_diagnostics,
            asset_cache,
            builds: self.builds.snapshot(),
            logs: self.logs.clone(),
            scripts_dirty: self.scripts.has_dirty_documents(),
            language_context: language_context(engine, &self.project),
            player_running,
            debugger: self.debugger.snapshot(),
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
            project_settings: project_settings(&self.project),
            project_manifest: self.project.manifest().clone(),
            project_revision: self.project_revision,
            recovery_available: self.recovery.is_available(),
            ..StudioSnapshot::default()
        };
        fill_scene_snapshot(engine, &mut snapshot);
        #[cfg(feature = "render_2d")]
        {
            snapshot.viewport_mode = engine
                .get_resource::<EditorState>()
                .map(|state| state.viewport_mode)
                .unwrap_or_default();
        }
        if let Ok(mut shared) = self.bridge.snapshot.lock() {
            *shared = snapshot;
        }
    }
}
