use super::*;

impl StudioEguiTool {
    pub(super) fn project_ui(&mut self, ui: &mut egui::Ui, snapshot: &StudioSnapshot) {
        if self.project_draft.is_none() || self.project_revision != snapshot.project_revision {
            self.project_draft = Some(snapshot.project_manifest.clone());
            self.project_revision = snapshot.project_revision;
        }
        let bridge = self.bridge.clone();
        let Some(manifest) = self.project_draft.as_mut() else { return; };

        ui.horizontal(|ui| {
            ui.heading("Project Settings");
            ui.separator();
            if ui.button("Save Settings").clicked() {
                bridge.push(StudioCommand::SaveProjectSettings(manifest.clone()));
            }
            if ui.button("Revert").clicked() {
                *manifest = snapshot.project_manifest.clone();
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::CollapsingHeader::new("Project")
                .default_open(true)
                .show(ui, |ui| {
                    settings_text(ui, "Name", &mut manifest.project.name);
                    settings_text(ui, "Version", &mut manifest.project.version);
                    settings_text(ui, "Engine version", &mut manifest.project.engine_version);
                    ui.label(format!("Project ID: {}", manifest.project.id));
                });

            egui::CollapsingHeader::new("Application")
                .default_open(true)
                .show(ui, |ui| {
                    settings_text(ui, "Window title", &mut manifest.application.title);
                    ui.horizontal(|ui| {
                        ui.label("Window size");
                        ui.add(egui::DragValue::new(&mut manifest.application.width).range(320..=16384));
                        ui.label("×");
                        ui.add(egui::DragValue::new(&mut manifest.application.height).range(240..=16384));
                    });
                    ui.checkbox(&mut manifest.application.resizable, "Resizable");
                    ui.checkbox(&mut manifest.application.fullscreen, "Fullscreen");
                    ui.checkbox(&mut manifest.application.cursor_grab, "Grab cursor while playing");
                    ui.checkbox(&mut manifest.application.cursor_visible, "Show cursor while playing");
                    let mut icon = manifest.application.icon.as_ref().map(ToString::to_string).unwrap_or_default();
                    if settings_text(ui, "Icon", &mut icon) {
                        manifest.application.icon = empty_project_path(&icon);
                    }
                });

            egui::CollapsingHeader::new("Runtime")
                .default_open(true)
                .show(ui, |ui| {
                    let mut main_scene = manifest.runtime.main_scene.to_string();
                    if settings_text(ui, "Main scene", &mut main_scene) {
                        if let Ok(path) = ProjectPath::new(main_scene.trim()) {
                            manifest.runtime.main_scene = path;
                        }
                    }
                    ui.label("Autoload scripts (one project-relative path per line)");
                    let mut autoloads = manifest.runtime.autoload_scripts.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
                    if ui.add(egui::TextEdit::multiline(&mut autoloads).desired_rows(3).desired_width(f32::INFINITY)).changed() {
                        manifest.runtime.autoload_scripts = autoloads.lines()
                            .filter_map(|line| ProjectPath::new(line.trim()).ok())
                            .collect();
                    }
                });

            egui::CollapsingHeader::new("Features").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut manifest.features.rendering, "Rendering");
                    ui.checkbox(&mut manifest.features.physics, "Physics");
                    ui.checkbox(&mut manifest.features.audio, "Audio");
                    ui.checkbox(&mut manifest.features.animation, "Animation");
                    ui.checkbox(&mut manifest.features.networking, "Networking");
                    ui.checkbox(&mut manifest.features.ui, "UI");
                    ui.checkbox(&mut manifest.features.scripting, "Scripting");
                });
            });

            egui::CollapsingHeader::new("Scripting").show(ui, |ui| {
                egui::ComboBox::from_id_source("project_script_language")
                    .selected_text(format!("{:?}", manifest.scripting.language))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut manifest.scripting.language, ScriptLanguage::Lua, "Lua");
                    });
                ui.checkbox(&mut manifest.scripting.hot_reload, "Hot reload");
                ui.checkbox(&mut manifest.scripting.fail_fast, "Stop runtime on script error");
                ui.horizontal(|ui| {
                    ui.label("Maximum errors per frame");
                    ui.add(egui::DragValue::new(&mut manifest.scripting.max_errors_per_frame).range(1..=10_000));
                });
            });

            egui::CollapsingHeader::new("Rendering").show(ui, |ui| {
                enum_combo(ui, "Backend", "render_backend", &mut manifest.rendering.backend, &[
                    (RenderingBackend::Auto, "Auto"),
                    (RenderingBackend::Wgpu, "WGPU"),
                    (RenderingBackend::SoftwareSdl, "Software SDL"),
                ]);
                ui.checkbox(&mut manifest.rendering.vsync, "VSync");
                ui.checkbox(&mut manifest.rendering.hdr, "HDR");
                ui.horizontal(|ui| {
                    ui.label("MSAA samples");
                    ui.add(egui::DragValue::new(&mut manifest.rendering.msaa_samples).range(1..=16));
                });
                ui.horizontal(|ui| {
                    ui.label("Render scale");
                    ui.add(egui::DragValue::new(&mut manifest.rendering.render_scale).range(0.25..=2.0).speed(0.05));
                });
                enum_combo(ui, "Shadow quality", "shadow_quality", &mut manifest.rendering.shadow_quality, &[
                    (ShadowQuality::Off, "Off"), (ShadowQuality::Low, "Low"),
                    (ShadowQuality::Medium, "Medium"), (ShadowQuality::High, "High"),
                    (ShadowQuality::Ultra, "Ultra"),
                ]);
                enum_combo(ui, "Global illumination", "gi_mode", &mut manifest.rendering.gi_mode, &[
                    (GiMode::None, "None"), (GiMode::Baked, "Baked"), (GiMode::Ddgi, "DDGI"),
                ]);
            });

            egui::CollapsingHeader::new("Physics").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Gravity");
                    for value in &mut manifest.physics.gravity {
                        ui.add(egui::DragValue::new(value).speed(0.1));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Fixed timestep");
                    ui.add(egui::DragValue::new(&mut manifest.physics.fixed_timestep).range(0.000_1..=1.0).speed(0.001));
                });
                ui.horizontal(|ui| {
                    ui.label("Maximum substeps");
                    ui.add(egui::DragValue::new(&mut manifest.physics.max_substeps).range(1..=64));
                });
            });

            egui::CollapsingHeader::new("Input Actions").show(ui, |ui| {
                let mut remove = None;
                for (name, action) in manifest.input.actions.iter_mut() {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.strong(name);
                            if ui.small_button("Remove").clicked() { remove = Some(name.clone()); }
                        });
                        binding_list(ui, "Keys", &mut action.keys);
                        binding_list(ui, "Mouse buttons", &mut action.mouse_buttons);
                        binding_list(ui, "Gamepad buttons", &mut action.gamepad_buttons);
                        ui.horizontal(|ui| {
                            ui.label("Dead zone");
                            ui.add(egui::DragValue::new(&mut action.dead_zone).range(0.0..=1.0).speed(0.01));
                        });
                        for axis in &mut action.axes {
                            ui.horizontal(|ui| {
                                settings_text(ui, "Axis", &mut axis.axis);
                                egui::ComboBox::from_id_source(("axis_direction", name, &axis.axis))
                                    .selected_text(match axis.direction { AxisDirection::Negative => "Negative", AxisDirection::Positive => "Positive" })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut axis.direction, AxisDirection::Negative, "Negative");
                                        ui.selectable_value(&mut axis.direction, AxisDirection::Positive, "Positive");
                                    });
                                ui.add(egui::DragValue::new(&mut axis.scale).speed(0.1));
                            });
                        }
                    });
                }
                if let Some(name) = remove { manifest.input.actions.remove(&name); }
                ui.horizontal(|ui| {
                    ui.label("New action");
                    let id = ui.id().with("new_input_action");
                    let mut name = ui.data_mut(|data| data.get_temp::<String>(id).unwrap_or_default());
                    let response = ui.text_edit_singleline(&mut name);
                    ui.data_mut(|data| data.insert_temp(id, name.clone()));
                    if ui.add_enabled(!name.trim().is_empty(), egui::Button::new("Add")).clicked() {
                        manifest.input.actions.entry(name.trim().to_owned()).or_insert_with(InputAction::default);
                        ui.data_mut(|data| data.insert_temp(id, String::new()));
                    }
                    if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) && !name.trim().is_empty() {
                        manifest.input.actions.entry(name.trim().to_owned()).or_insert_with(InputAction::default);
                        ui.data_mut(|data| data.insert_temp(id, String::new()));
                    }
                });
            });
        });
    }
}
