use super::*;

impl StudioEguiTool {
    pub(super) fn draw_toolbar(
        &mut self,
        ctx: &egui::Context,
        snapshot: &StudioSnapshot,
    ) -> egui::InnerResponse<()> {
        egui::TopBottomPanel::top("vetrace_studio_toolbar")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Vetrace Studio");
                    ui.separator();
                    self.draw_recovery_controls(ui, snapshot);
                    self.draw_file_menu(ui, snapshot);
                    self.draw_history_controls(ui, snapshot);
                    self.draw_player_controls(ui, snapshot);
                    self.draw_creation_controls(ui, snapshot);
                    self.draw_scene_status_and_navigation(ui, snapshot);
                });
            })
    }

    fn draw_recovery_controls(&self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        if !snapshot.recovery_available {
            return;
        }
        ui.colored_label(egui::Color32::YELLOW, "Unsaved recovery available");
        if ui.button("Recover").clicked() {
            self.command(StudioCommand::RecoverSession);
        }
        if ui.button("Discard").clicked() {
            self.command(StudioCommand::DiscardRecovery);
        }
        ui.separator();
    }

    fn draw_file_menu(&self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        ui.menu_button("File", |ui| {
            let scenes = snapshot.project_root.join("assets/scenes");
            let can_switch = !snapshot.dirty && !snapshot.scripts_dirty;
            if ui
                .add_enabled(can_switch, egui::Button::new("New Scene…"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&scenes)
                    .add_filter("Vetrace Scene", &["vscene"])
                    .set_file_name("new_scene.vscene")
                    .save_file()
                {
                    self.command(StudioCommand::NewScene(path));
                }
                ui.close_menu();
            }
            if ui
                .add_enabled(can_switch, egui::Button::new("Open Scene…"))
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&scenes)
                    .add_filter("Vetrace Scene", &["vscene"])
                    .pick_file()
                {
                    self.command(StudioCommand::OpenScene(path));
                }
                ui.close_menu();
            }
            if ui.button("Save Scene As…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_directory(&scenes)
                    .add_filter("Vetrace Scene", &["vscene"])
                    .set_file_name("scene.vscene")
                    .save_file()
                {
                    self.command(StudioCommand::SaveSceneAs(path));
                }
                ui.close_menu();
            }
            ui.separator();
            if ui.button("Set Current as Main Scene").clicked() {
                self.command(StudioCommand::SetCurrentSceneAsMain);
                ui.close_menu();
            }
        });
    }

    fn draw_history_controls(&mut self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        if ui.button("Save").clicked() {
            self.command(StudioCommand::SaveAllScripts);
            self.command(StudioCommand::SaveScene);
        }
        if ui
            .add_enabled(snapshot.can_undo, egui::Button::new("Undo"))
            .on_hover_text("Ctrl+Z")
            .clicked()
        {
            self.command(StudioCommand::Undo);
        }
        if ui
            .add_enabled(snapshot.can_redo, egui::Button::new("Redo"))
            .on_hover_text("Ctrl+Shift+Z")
            .clicked()
        {
            self.command(StudioCommand::Redo);
        }
        if ui.button("Reload").clicked() {
            if snapshot.dirty || snapshot.scripts_dirty {
                self.confirmation = Some(Confirmation::Reload);
            } else {
                self.command(StudioCommand::ReloadScene);
            }
        }
        ui.separator();
    }

    fn draw_player_controls(&mut self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        if !snapshot.player_running {
            if ui.button("▶ Play").clicked() {
                self.bottom_tab = BottomTab::Console;
                self.command(StudioCommand::PlayProject);
            }
            if ui
                .button("◆ Debug")
                .on_hover_text("Run with Lua debugger")
                .clicked()
            {
                self.bottom_tab = BottomTab::Scripts;
                self.command(StudioCommand::DebugProject);
            }
        } else {
            if ui.button("■ Stop").clicked() {
                self.command(StudioCommand::StopProject);
            }
            self.draw_debugger_controls(ui, snapshot);
        }
        if ui.button("Export").clicked() {
            self.bottom_tab = BottomTab::Build;
        }
        if ui.button("Script").clicked() {
            self.bottom_tab = BottomTab::Scripts;
        }
        ui.separator();
    }

    fn draw_debugger_controls(&self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        if !snapshot.debugger.connected {
            return;
        }
        if snapshot.debugger.paused.is_some() {
            for (label, command) in [
                ("Continue", LuaDebuggerCommand::Continue),
                ("Into", LuaDebuggerCommand::StepInto),
                ("Over", LuaDebuggerCommand::StepOver),
                ("Out", LuaDebuggerCommand::StepOut),
            ] {
                if ui.button(label).clicked() {
                    self.command(StudioCommand::DebugCommand(command));
                }
            }
        } else if ui.button("Pause").clicked() {
            self.command(StudioCommand::DebugCommand(LuaDebuggerCommand::Pause));
        }
    }

    fn draw_creation_controls(&self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        #[cfg(feature = "render_2d")]
        {
            let is_2d = snapshot.viewport_mode == vetrace_editor::EditorViewportMode::TwoD;
            if ui.selectable_label(is_2d, "2D").clicked() {
                self.command(StudioCommand::SetViewportMode(vetrace_editor::EditorViewportMode::TwoD));
            }
            if ui.selectable_label(!is_2d, "3D").clicked() {
                self.command(StudioCommand::SetViewportMode(vetrace_editor::EditorViewportMode::ThreeD));
            }
            ui.separator();
        }
        ui.menu_button("Add", |ui| {
            if ui.button("Empty Entity").clicked() {
                self.command(StudioCommand::SpawnEmpty);
                ui.close_menu();
            }
            #[cfg(feature = "render_2d")]
            if ui.button("Sprite 2D").clicked() {
                self.command(StudioCommand::SpawnSprite2D);
                ui.close_menu();
            }
            ui.separator();
            for (label, kind) in [
                ("Cube", vetrace_primitives::PrimitiveKind::Cube),
                ("Sphere", vetrace_primitives::PrimitiveKind::Sphere),
                ("Capsule", vetrace_primitives::PrimitiveKind::Capsule),
                ("Plane", vetrace_primitives::PrimitiveKind::Plane),
                ("Quad", vetrace_primitives::PrimitiveKind::Quad),
            ] {
                if ui.button(label).clicked() {
                    self.command(StudioCommand::SpawnPrimitive(kind));
                    ui.close_menu();
                }
            }
        });
        if ui.button("Delete").clicked() {
            self.command(StudioCommand::DeleteSelected);
        }
        ui.separator();
    }

    fn draw_scene_status_and_navigation(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &StudioSnapshot,
    ) {
        let dirty = if snapshot.dirty || snapshot.scripts_dirty {
            " • modified"
        } else {
            ""
        };
        ui.label(format!("{}{}", snapshot.scene_path, dirty));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Quit").clicked() {
                if snapshot.dirty || snapshot.scripts_dirty {
                    self.confirmation = Some(Confirmation::Quit);
                } else {
                    self.command(StudioCommand::Quit);
                }
            }
            if ui.button("Projects").clicked() {
                if snapshot.dirty || snapshot.scripts_dirty {
                    self.confirmation = Some(Confirmation::ProjectManager);
                } else {
                    self.command(StudioCommand::OpenProjectManager);
                }
            }
            ui.label(&snapshot.project_name);
        });
    }
}
