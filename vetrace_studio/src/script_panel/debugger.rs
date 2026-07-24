use super::*;

impl ScriptEditorPanel {
    pub(super) fn debugger_ui(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &StudioSnapshot,
        commands: &mut Vec<StudioCommand>,
    ) {
        if !self.watches_initialized {
            self.watches_text = snapshot.debugger.watches.join("\n");
            self.watches_initialized = true;
        }
        egui::CollapsingHeader::new("Lua Debugger")
            .default_open(snapshot.debugger.paused.is_some())
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if !snapshot.player_running {
                        if ui.button("Start Debugging").clicked() {
                            commands.push(StudioCommand::DebugProject);
                        }
                    } else if snapshot.debugger.connected {
                        if snapshot.debugger.paused.is_some() {
                            if ui.button("Continue").clicked() { commands.push(StudioCommand::DebugCommand(LuaDebuggerCommand::Continue)); }
                            if ui.button("Step Into").clicked() { commands.push(StudioCommand::DebugCommand(LuaDebuggerCommand::StepInto)); }
                            if ui.button("Step Over").clicked() { commands.push(StudioCommand::DebugCommand(LuaDebuggerCommand::StepOver)); }
                            if ui.button("Step Out").clicked() { commands.push(StudioCommand::DebugCommand(LuaDebuggerCommand::StepOut)); }
                        } else if ui.button("Pause").clicked() {
                            commands.push(StudioCommand::DebugCommand(LuaDebuggerCommand::Pause));
                        }
                    } else {
                        ui.label("Waiting for debugger connection…");
                    }
                    let mut break_on_error = snapshot.debugger.break_on_error;
                    if ui.checkbox(&mut break_on_error, "Break on error").changed() {
                        commands.push(StudioCommand::SetBreakOnError(break_on_error));
                    }
                });
                ui.horizontal_top(|ui| {
                    ui.label("Watches");
                    ui.add(egui::TextEdit::multiline(&mut self.watches_text)
                        .desired_width(260.0)
                        .desired_rows(2)
                        .hint_text("self.health\nself.transform"));
                    if ui.button("Apply watches").clicked() {
                        commands.push(StudioCommand::SetDebuggerWatches(
                            self.watches_text.lines().map(str::to_owned).collect(),
                        ));
                    }
                });
                let Some(paused) = snapshot.debugger.paused.as_ref() else { return; };
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Paused: {}:{} — {} / {}", paused.path, paused.line, paused.reason, paused.callback),
                );
                ui.columns(3, |columns| {
                    columns[0].strong("Call stack");
                    for frame in &paused.stack {
                        columns[0].label(format!("{}  {}:{}", frame.name, frame.source, frame.line.unwrap_or(0)));
                    }
                    columns[1].strong("Self / instance");
                    for variable in &paused.locals {
                        columns[1].label(format!("{} = {}", variable.name, debug_value_summary(&variable.value)));
                    }
                    columns[2].strong("Watches");
                    for variable in &paused.watches {
                        columns[2].label(format!("{} = {}", variable.name, debug_value_summary(&variable.value)));
                    }
                });
            });
    }

}
