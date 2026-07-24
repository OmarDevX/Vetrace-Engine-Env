use super::*;

impl StudioPlugin {
    pub(super) fn apply_debug_command(&mut self, engine: &mut Engine, command: StudioCommand) {
        match command {
            StudioCommand::PlayProject => self.play_project(engine, false),
            StudioCommand::DebugProject => self.play_project(engine, true),
            StudioCommand::DebugCommand(command) => {
                if let Err(error) = self.player.send_debugger_command(&command) { self.log(error); }
            }
            StudioCommand::ToggleBreakpoint { path, line } => {
                match self.debugger.toggle_breakpoint(&path, line, &self.project) {
                    Ok(()) => {
                        if self.debugger.snapshot().connected {
                            let command = self.debugger.breakpoint_command();
                            if let Err(error) = self.player.send_debugger_command(&command) { self.log(error); }
                        }
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::SetDebuggerWatches(watches) => {
                match self.debugger.set_watches(watches) {
                    Ok(()) => {
                        if self.debugger.snapshot().connected {
                            let command = self.debugger.watches_command();
                            if let Err(error) = self.player.send_debugger_command(&command) { self.log(error); }
                        }
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::SetBreakOnError(enabled) => {
                match self.debugger.set_break_on_error(enabled) {
                    Ok(()) => {
                        if self.debugger.snapshot().connected {
                            let command = self.debugger.break_on_error_command();
                            if let Err(error) = self.player.send_debugger_command(&command) { self.log(error); }
                        }
                    }
                    Err(error) => self.log(error),
                }
            }
            StudioCommand::StopProject => match self.player.stop() {
                Ok(()) => {
                    self.debugger.reset_connection();
                    self.status = "Game stopped".to_string();
                    self.log("Stopped vetrace-player");
                }
                Err(error) => self.log(error),
            },
            _ => unreachable!("non-debug command routed to debug handler"),
        }
    }
}
