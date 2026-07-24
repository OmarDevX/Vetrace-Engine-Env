use super::*;

impl StudioPlugin {
    pub(super) fn collect_player_output(&mut self) {
        for event in self.player.drain_events() {
            match event {
                PlayerProcessEvent::Output(line) => {
                    self.scripts.ingest_player_output(&line.text);
                    let prefix = match line.stream {
                        PlayerOutputStream::Stdout => "[Game]",
                        PlayerOutputStream::Stderr => "[Game stderr]",
                    };
                    self.log(format!("{prefix} {}", line.text));
                }
                PlayerProcessEvent::Debugger(event) => self.handle_debugger_event(event),
            }
        }
    }

    fn handle_debugger_event(&mut self, event: LuaDebuggerEvent) {
        self.debugger.handle_event(&event);
        match &event {
            LuaDebuggerEvent::Ready => {
                for command in self.debugger.configuration_commands() {
                    if let Err(error) = self.player.send_debugger_command(&command) { self.log(error); }
                }
                self.status = "Lua debugger connected".to_owned();
            }
            LuaDebuggerEvent::Paused { state } => {
                self.status = format!("Paused at {}:{} ({})", state.path, state.line, state.reason);
                if let Ok(project_path) = vetrace_project::ProjectPath::new(&state.path) {
                    if let Ok(path) = self.project.paths().resolve_existing(&project_path) {
                        if let Err(error) = self.scripts.open(path, Some(state.line)) { self.log(error); }
                    }
                }
            }
            LuaDebuggerEvent::Resumed => self.status = "Debugger resumed".to_owned(),
            LuaDebuggerEvent::Error { path, line, message, .. } => {
                self.log(format!("[Lua debugger] {path}:{}: {message}", line.unwrap_or(1)));
            }
        }
    }

    pub(super) fn play_project(&mut self, engine: &mut Engine, debug: bool) {
        if self.scripts.has_dirty_documents() && !self.save_all_scripts() { return; }
        let (play_scene, cleanup_file) = match save_temporary_play_scene(engine, &self.project) {
            Ok(value) => value,
            Err(error) => { self.log(error); return; }
        };
        self.debugger.reset_connection();
        match self.player.start(
            self.project.root(),
            Some(&play_scene),
            Some(cleanup_file.clone()),
            debug,
        ) {
            Ok(()) => {
                if debug {
                    // Commands can be written before the player emits `Ready`;
                    // the player intentionally waits for all three before it
                    // starts autoloads or entity `ready` callbacks.
                    for command in self.debugger.configuration_commands() {
                        if let Err(error) = self.player.send_debugger_command(&command) {
                            self.log(error);
                            let _ = self.player.stop();
                            return;
                        }
                    }
                }
                self.status = if debug { "Game running under debugger" } else { "Game running" }.to_owned();
                self.log(format!(
                    "Launched isolated play scene {} with vetrace-player{}",
                    play_scene,
                    if debug { " debugger" } else { "" },
                ));
            }
            Err(error) => {
                let _ = std::fs::remove_file(cleanup_file);
                self.log(error);
            }
        }
    }

    pub(super) fn open_project_manager(&mut self, engine: &mut Engine) {
        if let Err(error) = self.player.stop() {
            self.log(error);
        }
        match launch_project_manager() {
            Ok(()) => engine.stop(),
            Err(error) => self.log(error),
        }
    }
}
