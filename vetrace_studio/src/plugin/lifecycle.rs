use super::*;

impl Plugin for StudioPlugin {
    fn name(&self) -> &'static str { "studio" }

    fn dependencies(&self) -> Vec<&'static str> {
        let mut dependencies = vec!["render"];
        #[cfg(feature = "render_2d")]
        dependencies.push("render_2d");
        dependencies
    }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        if !engine.contains_resource::<EguiToolRegistry>() {
            engine.insert_resource(EguiToolRegistry::new());
        }
        if let Some(registry) = engine.get_resource::<EguiToolRegistry>().cloned() {
            registry.register(StudioEguiTool::new(self.bridge.clone(), self.scripts.clone()));
        }
        if !engine.contains_resource::<EditorPointerCapture>() {
            engine.insert_resource(EditorPointerCapture::default());
        }
        if !engine.contains_resource::<EditorKeyboardCapture>() {
            engine.insert_resource(EditorKeyboardCapture::default());
        }
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.title = format!("Vetrace Studio — {}", self.project.manifest().project.name);
            settings.width = settings.width.max(1280);
            settings.height = settings.height.max(720);
            settings.cursor_grab = false;
            settings.cursor_visible = true;
            settings.draw_bounds = true;
        }
        if !engine.contains_resource::<StudioCameraState>() {
            engine.insert_resource(StudioCameraState::default());
        }
        for message in self.assets.initialize(&self.project) { self.log(message); }
        self.status = "Ready".to_string();
        self.log(format!("Opened project {}", self.project.root().display()));
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        self.initialize_history(engine);
        self.collect_player_output();
        for message in self.assets.update(dt) {
            self.status = message.clone();
            self.log(message);
        }
        if let Some(result) = self.builds.update() {
            match result {
                Ok(report) => {
                    self.status = format!("Exported {}", report.output_directory.display());
                    self.log(format!(
                        "Export complete: {} entries, {} bytes, package {}",
                        report.package_entries,
                        report.package_bytes,
                        report.package.display(),
                    ));
                    for warning in report.warnings {
                        self.log(format!("Export warning: {warning}"));
                    }
                }
                Err(error) => {
                    self.status = "Export failed".to_owned();
                    self.log(format!("Export failed: {error}"));
                }
            }
        }

        let egui_pointer_captured = self
            .bridge
            .pointer_captured
            .lock()
            .map(|captured| *captured)
            .unwrap_or(false);
        let mouse_position = engine
            .get_resource::<InputState>()
            .map(InputState::mouse_position)
            .unwrap_or((0.0, 0.0));
        let viewport_rect = self
            .bridge
            .viewport_rect
            .lock()
            .ok()
            .and_then(|rect| *rect);
        if let Some(bounds) = engine.get_resource_mut::<EditorViewportBounds>() {
            bounds.0 = viewport_rect;
        } else {
            engine.insert_resource(EditorViewportBounds(viewport_rect));
        }
        let pointer_outside_viewport = EditorViewportBounds(viewport_rect)
            .blocks_pointer(mouse_position.0, mouse_position.1);
        if let Some(capture) = engine.get_resource_mut::<EditorPointerCapture>() {
            capture.0 = egui_pointer_captured;
        }
        let keyboard_captured = self
            .bridge
            .keyboard_captured
            .lock()
            .map(|captured| *captured)
            .unwrap_or(false);
        if let Some(capture) = engine.get_resource_mut::<EditorKeyboardCapture>() {
            capture.0 = keyboard_captured;
        }
        update_studio_camera(
            engine,
            egui_pointer_captured || pointer_outside_viewport || keyboard_captured,
            dt,
        );

        let player_running_before_commands = self.player.is_running();
        let mut commands = self.bridge.drain_commands();
        if !keyboard_captured {
            append_keyboard_shortcuts(engine, player_running_before_commands, &mut commands);
        }
        for command in commands {
            self.apply_command(engine, command);
        }

        self.track_editor_changes(engine);
        let primary_pointer_down = engine
            .get_resource::<InputState>()
            .map(|input| input.is_mouse_button_down("Left"))
            .unwrap_or(false);
        self.tick_history(engine, dt, keyboard_captured || primary_pointer_down);

        self.scripts.maintenance();

        if self.recovery.tick(dt) {
            if self.dirty || self.scripts.has_dirty_documents() {
                match capture_authored_scene(engine) {
                    Ok(snapshot) => {
                        let path = active_scene_project_path(engine, &self.project);
                        let scripts = self.scripts.recovery_scripts(&self.project);
                        if let Err(error) = self.recovery.save(
                            &self.project,
                            path,
                            snapshot.document,
                            scripts,
                        ) {
                            self.log(format!("Autosave failed: {error}"));
                        } else {
                            self.status = "Autosaved recovery session".to_owned();
                        }
                    }
                    Err(error) => self.log(format!("Autosave failed: {error}")),
                }
            }
        }

        self.collect_player_output();
        let player_running = self.player.is_running();
        if !player_running {
            self.debugger.reset_connection();
            if let Some(status) = self.player.take_exit_status() {
                self.log(format!("[Game] process exited with {status}"));
            }
            if self.status == "Game running" {
                self.status = "Game exited".to_string();
                self.log("vetrace-player exited");
            }
        }
        self.refresh_snapshot(engine, player_running);
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}
