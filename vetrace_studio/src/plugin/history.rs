use super::*;

impl StudioPlugin {
    pub(super) fn initialize_history(&mut self, engine: &mut Engine) {
        if self.history_ready { return; }
        match capture_authored_scene(engine) {
            Ok(snapshot) => {
                self.saved_fingerprint = snapshot.fingerprint().to_vec();
                self.history.reset(snapshot);
                self.history_ready = true;
                self.transform_signature = Some(authored_transform_signature(engine));
            }
            Err(error) => self.log(error),
        }
    }

    pub(super) fn mark_scene_changed(&mut self, label: impl Into<String>) {
        self.dirty = true;
        self.history_pending = true;
        self.history_idle_seconds = 0.0;
        self.history_label = label.into();
    }

    pub(super) fn record_current_history(&mut self, engine: &mut Engine) -> bool {
        self.initialize_history(engine);
        let snapshot = match capture_authored_scene(engine) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.log(error);
                return false;
            }
        };
        let changed = self.history.record(snapshot.clone());
        self.history_pending = false;
        self.history_idle_seconds = 0.0;
        self.dirty = snapshot.fingerprint() != self.saved_fingerprint.as_slice();
        changed
    }

    fn commit_pending_history(&mut self, engine: &mut Engine) {
        if !self.history_pending { return; }
        let label = std::mem::take(&mut self.history_label);
        if self.record_current_history(engine) && !label.is_empty() {
            self.status = label;
        }
    }

    pub(super) fn tick_history(&mut self, engine: &mut Engine, dt: f32, editing_active: bool) {
        if !self.history_pending { return; }
        if editing_active {
            self.history_idle_seconds = 0.0;
            return;
        }
        self.history_idle_seconds += dt.max(0.0).min(0.1);
        if self.history_idle_seconds >= 0.2 {
            self.commit_pending_history(engine);
        }
    }

    fn restore_history_snapshot(
        &mut self,
        engine: &mut Engine,
        snapshot: AuthoredSceneSnapshot,
        action: &str,
    ) {
        match restore_authored_scene(engine, &self.project, &snapshot) {
            Ok(()) => {
                self.history_pending = false;
                self.history_idle_seconds = 0.0;
                self.transform_signature = Some(authored_transform_signature(engine));
                self.dirty = snapshot.fingerprint() != self.saved_fingerprint.as_slice();
                self.status = action.to_string();
                self.log(action);
            }
            Err(error) => self.log(error),
        }
    }

    pub(super) fn undo(&mut self, engine: &mut Engine) {
        self.record_current_history(engine);
        match self.history.undo() {
            Some(snapshot) => self.restore_history_snapshot(engine, snapshot, "Undo"),
            None => self.status = "Nothing to undo".to_string(),
        }
    }

    pub(super) fn redo(&mut self, engine: &mut Engine) {
        self.record_current_history(engine);
        match self.history.redo() {
            Some(snapshot) => self.restore_history_snapshot(engine, snapshot, "Redo"),
            None => self.status = "Nothing to redo".to_string(),
        }
    }
}
