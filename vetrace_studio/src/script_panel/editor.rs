use super::*;

impl ScriptEditorPanel {
    pub(super) fn editor_ui(&mut self, ui: &mut egui::Ui, state: &mut StudioScriptState, active_index: usize, snapshot: &StudioSnapshot, commands: &mut Vec<StudioCommand>) {
        let language_id = state.workspace.documents()[active_index].language_id.clone();
        let service = state.workspace.registry().get(&language_id);
        let document = &state.workspace.documents()[active_index];
        let highlights = document.highlights.clone();
        let diagnostics = StudioScripts::syntax_and_runtime_diagnostics(state, active_index);
        let layout_diagnostics = diagnostics.clone();
        let mut text = document.text.clone();
        let line_count = text.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let document_path = state.workspace.documents()[active_index].path.clone();
        let debug_path = document_path
            .strip_prefix(&snapshot.project_root)
            .ok()
            .map(|path| path.to_string_lossy().replace('\\', "/"));
        let breakpoints = debug_path
            .as_ref()
            .and_then(|path| snapshot.debugger.breakpoints.get(path))
            .cloned()
            .unwrap_or_default();
        let paused_line = snapshot.debugger.paused.as_ref().and_then(|paused| {
            debug_path
                .as_ref()
                .is_some_and(|path| path == &paused.path)
                .then_some(paused.line)
        });
        let active_line = paused_line.or(self.target_line).unwrap_or(self.active_line).max(1);
        let mut layouter = move |ui: &egui::Ui, buffer: &str, wrap_width: f32| {
            let mut job = highlighted_layout_job(
                buffer,
                &highlights,
                &layout_diagnostics,
                active_line,
            );
            job.wrap.max_width = wrap_width;
            ui.fonts(|fonts| fonts.layout_job(job))
        };

        let editor_height = ui.available_height().max(64.0);
        egui::ScrollArea::both()
            .id_source("vetrace_script_editor_scroll")
            .max_height(editor_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_min_width(46.0);
                        for line in 1..=line_count {
                            let diagnostic = diagnostics.iter().find(|diagnostic| diagnostic.position.line == line);
                            let has_breakpoint = breakpoints.contains(&line);
                            if ui.small_button(if has_breakpoint { "●" } else { "·" })
                                .on_hover_text(if has_breakpoint { "Remove breakpoint" } else { "Add breakpoint" })
                                .clicked()
                            {
                                commands.push(StudioCommand::ToggleBreakpoint {
                                    path: document_path.clone(),
                                    line,
                                });
                            }
                            let color = if paused_line == Some(line) {
                                egui::Color32::YELLOW
                            } else if diagnostic.is_some() {
                                egui::Color32::from_rgb(235, 90, 90)
                            } else if line == active_line {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::GRAY
                            };
                            let response = ui.label(egui::RichText::new(format!("{line:>4}")).monospace().color(color));
                            let response = if let Some(diagnostic) = diagnostic {
                                response.on_hover_text(&diagnostic.message)
                            } else {
                                response
                            };
                            if self.target_line == Some(line) {
                                response.scroll_to_me(Some(egui::Align::Center));
                                self.target_line = None;
                            }
                        }
                    });
                    let old_text = state.workspace.documents()[active_index].text.clone();
                    let mut output = egui::TextEdit::multiline(&mut text)
                        .id_source("vetrace_script_editor_text")
                        .code_editor()
                        .font(egui::TextStyle::Monospace)
                        .desired_width(900.0)
                        .desired_rows(1)
                        .lock_focus(true)
                        .layouter(&mut layouter)
                        .show(ui);
                    if let Some(cursor) = output.cursor_range {
                        self.cursor_byte = char_index_to_byte(&text, cursor.primary.ccursor.index);
                        self.active_line = line_for_offset(&text, self.cursor_byte);
                    }
                    if let Some(target) = self.pending_cursor_byte.take() {
                        self.cursor_byte = target.min(text.len());
                        self.active_line = line_for_offset(&text, self.cursor_byte);
                        set_text_edit_cursor(ui, &mut output, &text, self.cursor_byte);
                    }
                    if output.response.changed() {
                        let (assisted, cursor) = apply_editor_assists(&old_text, text, self.cursor_byte);
                        text = assisted;
                        self.cursor_byte = cursor.min(text.len());
                        self.active_line = line_for_offset(&text, self.cursor_byte);
                        set_text_edit_cursor(ui, &mut output, &text, self.cursor_byte);
                        state.workspace.documents_mut()[active_index].set_text(text.clone());
                        self.last_edit = Some(Instant::now());
                        let prefix = &text[..self.cursor_byte.min(text.len())];
                        self.completion_open = prefix.ends_with('.') || prefix.ends_with(':');
                    }
                    let path = state.workspace.documents()[active_index].path.clone();
                    state.view_states.insert(path, ScriptViewState {
                        cursor_byte: self.cursor_byte,
                        line: self.active_line,
                    });
                });
            });

        if let Some(service) = &service {
            let document = &state.workspace.documents()[active_index];
            if let Some(signature) = service.signature_help(&document.text, self.cursor_byte) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("ƒ").strong());
                    for (index, parameter) in signature.parameters.iter().enumerate() {
                        if index > 0 { ui.label(","); }
                        let text = if index == signature.active_parameter {
                            egui::RichText::new(parameter).strong().color(egui::Color32::LIGHT_BLUE)
                        } else {
                            egui::RichText::new(parameter)
                        };
                        ui.label(text);
                    }
                    if let Some(documentation) = signature.documentation { ui.weak(documentation); }
                });
            }
        }

        let needs_analysis = state.workspace.documents()[active_index].diagnostics_revision
            != state.workspace.documents()[active_index].revision;
        if needs_analysis && self.last_edit.is_some_and(|instant| instant.elapsed() >= Duration::from_millis(250)) {
            let _ = state.workspace.analyze(active_index);
            self.last_edit = None;
        } else if service.is_none() {
            ui.label("Language service unavailable");
        }
    }

}
