use vetrace_engine::{ecs::Entity, engine::engine::Engine, scene::object::Object};
use vetrace_engine::systems::gizmo::EditorGizmoMode;
use egui::{ComboBox, Context, Key, Margin, ScrollArea, Slider, TextEdit, TopBottomPanel, Ui, Window};
use rfd::FileDialog;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;
use transform_gizmo_egui::GizmoOrientation;

use super::{SandboxWindow, NewField};

pub struct MainWindow {
    pub show_sandbox_window: bool,
    pub selected_component: String,
    pub selected_entities: Vec<Entity>,
    pub new_component_name: String,
    pub custom_component_name: String,
    pub custom_fields: Vec<NewField>,
    pub rename_buffers: HashMap<Entity, String>,
    pub left_rect: Option<egui::Rect>,
    pub right_rect: Option<egui::Rect>,
    pub bottom_rect: Option<egui::Rect>,
    pub top_rect: Option<egui::Rect>,
    pub file_explorer_path: String,
    pub gizmo_hovered: bool,
    pub gizmo_mode: EditorGizmoMode,
    pub gizmo_orientation: GizmoOrientation,
}

impl MainWindow {
    pub fn new() -> Self {
        Self {
            show_sandbox_window: false,
            selected_component: String::new(),
            selected_entities: Vec::new(),
            new_component_name: String::new(),
            custom_component_name: String::new(),
            custom_fields: Vec::new(),
            rename_buffers: HashMap::new(),
            left_rect: None,
            right_rect: None,
            bottom_rect: None,
            top_rect: None,
            file_explorer_path: ".".to_string(),
            gizmo_hovered: false,
            gizmo_mode: EditorGizmoMode::Translate,
            gizmo_orientation: GizmoOrientation::Local,
        }
    }

    pub fn ui(&mut self, ctx: &Context, sandbox: &mut SandboxWindow, engine: &mut Engine) {
        self.top_panel_ui(ctx, engine);
        self.left_panel_ui(ctx, engine);
        self.right_panel_ui(ctx, engine);
        self.file_explorer_ui(ctx, engine);

        if self.show_sandbox_window {
            Window::new("Sandbox")
                .collapsible(true)
                .resizable(true)
                .show(ctx, |ui| {
                    sandbox.ui(ctx, ui, engine);
                });
        }
    }

    pub fn blur_rects(&self) -> Vec<egui::Rect> {
        let mut r = Vec::new();
        if let Some(rect) = self.left_rect { r.push(rect); }
        if let Some(rect) = self.right_rect { r.push(rect); }
        if let Some(rect) = self.bottom_rect { r.push(rect); }
        if let Some(rect) = self.top_rect { r.push(rect); }
        r
    }

    fn top_panel_ui(&mut self, ctx: &Context, _engine: &mut Engine) {
        let resp = TopBottomPanel::top("top_panel")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 60, 210))
                    .rounding(8.0)
                    .inner_margin(Margin::same(6.0))
                    .outer_margin(Margin {
                        left: 10.0,
                        right: 10.0,
                        top: 10.0,
                        bottom: 0.0,
                    }),
            )
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ComboBox::from_id_source("gizmo_mode_combo")
                        .selected_text(match self.gizmo_mode {
                            EditorGizmoMode::Translate => "Move",
                            EditorGizmoMode::Rotate => "Rotate",
                            EditorGizmoMode::Scale => "Scale",
                            EditorGizmoMode::Omni => "Omni",
                            EditorGizmoMode::Arcball => "Arcball",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.gizmo_mode, EditorGizmoMode::Translate, "Move");
                            ui.selectable_value(&mut self.gizmo_mode, EditorGizmoMode::Rotate, "Rotate");
                            ui.selectable_value(&mut self.gizmo_mode, EditorGizmoMode::Scale, "Scale");
                            ui.selectable_value(&mut self.gizmo_mode, EditorGizmoMode::Omni, "Omni");
                            ui.selectable_value(&mut self.gizmo_mode, EditorGizmoMode::Arcball, "Arcball");
                        });
                    ui.separator();
                    ComboBox::from_id_source("gizmo_orientation_combo")
                        .selected_text(match self.gizmo_orientation {
                            GizmoOrientation::Local => "Local",
                            GizmoOrientation::Global => "Global",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.gizmo_orientation, GizmoOrientation::Local, "Local");
                            ui.selectable_value(&mut self.gizmo_orientation, GizmoOrientation::Global, "Global");
                        });
                });
            });
        self.top_rect = Some(resp.response.rect);
    }

    fn left_panel_ui(&mut self, ctx: &Context, engine: &mut Engine) {
        let resp = egui::SidePanel::left("left_panel")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 60, 210))
                    .rounding(8.0)
                    .inner_margin(Margin::same(6.0))
                    .outer_margin(Margin { left: 10.0, right: 0.0, top: 10.0, bottom: 10.0 }),
            )
            .resizable(true)
            .max_width(350.0)
            .min_width(150.0)
            .default_width(250.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("✢ Vetracer Engine");
                    });
                    ui.horizontal(|ui| {
                        if engine.paused {
                            if ui.button("Start").clicked() { engine.resume(); }
                        } else {
                            if ui.button("Stop").clicked() { engine.pause(); }
                        }
                        if ui.button("Restart").clicked() { engine.restart(); }
                    });
                    ui.separator();

                    if ui.button("Toggle Sandbox Window").clicked() {
                        self.show_sandbox_window = !self.show_sandbox_window;
                    }

                    ui.separator();
                    if ui.button("Load Scene").clicked() {
                        if let Some(path) = FileDialog::new().add_filter("scene", &["json"]).pick_file() {
                            if let Some(p) = path.to_str() {
                                if let Err(e) = engine.load_scene_from_file(p) {
                                    eprintln!("Failed to load scene: {}", e);
                                }
                            }
                        }
                    }
                    if ui.button("Save Scene").clicked() {
                        if let Some(path) = FileDialog::new().add_filter("scene", &["json"]).save_file() {
                            if let Some(p) = path.to_str() {
                                if let Err(e) = engine.save_scene_to_file(p) {
                                    eprintln!("Failed to save scene: {}", e);
                                }
                            }
                        }
                    }

                    ui.separator();
                    if ui.button("Deselect").clicked() {
                        self.selected_entities.clear();
                    }

                    ui.separator();
                    ui.collapsing("Entities", |ui| {
                        let entities: Vec<_> = engine.world.entities().to_vec();
                        for ent in entities {
                            let selected = self.selected_entities.contains(&ent);
                            let name = engine.get_entity_name(ent).unwrap_or("Unnamed").to_string();
                            let response = ui.selectable_label(selected, name);
                            if response.clicked() {
                                if engine.input.is_key_down(Keycode::LCtrl) || engine.input.is_key_down(Keycode::RCtrl) {
                                    if let Some(i) = self.selected_entities.iter().position(|e| *e == ent) {
                                        self.selected_entities.remove(i);
                                    } else {
                                        self.selected_entities.push(ent);
                                    }
                                } else {
                                    self.selected_entities.clear();
                                    self.selected_entities.push(ent);
                                }
                            }
                            response.context_menu(|ui| {
                                if ui.button("Duplicate").clicked() {
                                    engine.duplicate_entity(ent);
                                    ui.close_menu();
                                }
                                if ui.button("Remove").clicked() {
                                    engine.delete_entity(ent);
                                    if let Some(i) = self.selected_entities.iter().position(|e| *e == ent) {
                                        self.selected_entities.remove(i);
                                    }
                                    ui.close_menu();
                                }
                                ui.separator();
                                let entry = self.rename_buffers.entry(ent).or_insert_with(|| {
                                    engine.get_entity_name(ent).unwrap_or("").to_string()
                                });
                                ui.label("Rename:");
                                let rename = ui.text_edit_singleline(entry);
                                if rename.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                    engine.rename_entity(ent, entry);
                                    self.rename_buffers.remove(&ent);
                                    ui.close_menu();
                                }
                            });
                        }
                    });

                    ui.separator();
                    use egui::special_emojis::GITHUB;
                    ui.hyperlink_to(format!("{GITHUB} Resource Code"), "https://github.com/OmarDevX");
                    ui.separator();
                });
            });
        self.left_rect = Some(resp.response.rect);
    }

    fn right_panel_ui(&mut self, ctx: &Context, engine: &mut Engine) {
        let resp = egui::SidePanel::right("right_panel")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 60, 210))
                    .rounding(8.0)
                    .inner_margin(Margin::same(6.0))
                    .outer_margin(Margin { left: 0.0, right: 10.0, top: 10.0, bottom: 10.0 }),
            )
            .resizable(true)
            .max_width(550.0)
            .min_width(150.0)
            .default_width(250.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Entity Properties");
                    });
                    ui.separator();

                    if self.selected_entities.len() == 1 {
                        let entity = self.selected_entities[0];
                        ui.label(format!("Editing Entity {}", entity.0));
                        let component_names = engine.list_components_entity(entity);
                        let mut editors = vec![];
                        for name in component_names {
                            if let Some(editor_fn) = engine.component_editors.get(&name) {
                                editors.push((name.clone(), editor_fn.clone()));
                            }
                        }

                        for (name, editor) in editors {
                            ui.collapsing(name, |ui| {
                                editor(engine, entity, ui);
                            });
                        }

                        ui.separator();
                        ui.label("Add Component:");

                        let mut names: Vec<_> = engine.component_adders.keys().cloned().collect();
                        for g in &engine.generated_components {
                            if !names.contains(g) {
                                names.push(g.clone());
                            }
                        }
                        names.sort();
                        if self.selected_component.is_empty() && !names.is_empty() {
                            self.selected_component = names[0].clone();
                        }

                        ui.horizontal(|ui| {
                            ComboBox::from_id_source("component_select")
                                .selected_text(&self.selected_component)
                                .show_ui(ui, |ui| {
                                    for name in &names {
                                        ui.selectable_value(&mut self.selected_component, name.clone(), name);
                                    }
                                });

                            if ui.button("Add").clicked() {
                                if let Some(f) = engine.component_adders.get(&self.selected_component).cloned() {
                                    f(engine, entity);
                                }
                            }
                            if ui.button("Remove").clicked() {
                                if let Some(f) = engine.component_removers.get(&self.selected_component).cloned() {
                                    f(engine, entity);
                                }
                            }
                        });

                    } else if self.selected_entities.is_empty() {
                        ui.label("No entity selected");
                    } else {
                        ui.label(format!("{} entities selected", self.selected_entities.len()));
                    }

                    ui.separator();
                    ui.label("Create Custom Behaviour:");
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut self.new_component_name).desired_width(120.0));
                        if ui.button("Create").clicked() {
                            if !self.new_component_name.is_empty() {
                                engine.create_custom_component(&self.new_component_name);
                            }
                        }
                        if ui.button("Reload Scripts").clicked() {
                            engine.reload_scripts();
                        }
                        if ui.button("Reload Behaviours").clicked() {
                            engine.reload_component_behaviours();
                        }
                    });

                    ui.separator();
                    ui.label("Create Custom Component:");
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut self.custom_component_name).desired_width(120.0));
                        if ui.button("Add Field").clicked() {
                            self.custom_fields.push(NewField { name: String::new(), ty_index: 0, default: String::new() });
                        }
                        if ui.button("Generate").clicked() {
                            if !self.custom_component_name.is_empty() {
                                let fields: Vec<(String, String, String)> = self.custom_fields.iter().map(|f| {
                                    let ty = ["f32", "i32", "bool"][f.ty_index].to_string();
                                    (f.name.clone(), ty, f.default.clone())
                                }).collect();
                                engine.generate_component_file(&self.custom_component_name, &fields);
                                engine.reload_component_behaviours();
                                engine.update_generated_components();
                            }
                        }
                    });
                    for field in &mut self.custom_fields {
                        ui.horizontal(|ui| {
                            ui.add(TextEdit::singleline(&mut field.name).hint_text("name").desired_width(80.0));
                            ComboBox::from_id_source(format!("type_{}", field.name))
                                .selected_text(["f32", "i32", "bool"][field.ty_index])
                                .show_ui(ui, |ui| {
                                    for (i, t) in ["f32", "i32", "bool"].iter().enumerate() {
                                        ui.selectable_value(&mut field.ty_index, i, *t);
                                    }
                                });
                            ui.add(TextEdit::singleline(&mut field.default).hint_text("default").desired_width(60.0));
                        });
                    }
                });
            });
        self.right_rect = Some(resp.response.rect);
    }

    fn file_explorer_ui(&mut self, ctx: &Context, engine: &mut Engine) {
        use std::fs;
        let resp = TopBottomPanel::bottom("file_explorer")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 60, 210))
                    .rounding(8.0)
                    .inner_margin(Margin::same(6.0))
                    .outer_margin(Margin::same(10.0)),
            )
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Up").clicked() {
                        if let Some(p) = std::path::Path::new(&self.file_explorer_path).parent() {
                            self.file_explorer_path = p.to_string_lossy().to_string();
                        }
                    }
                    ui.label(&self.file_explorer_path);
                });
                ui.separator();
                if let Ok(entries) = fs::read_dir(&self.file_explorer_path) {
                    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
                    entries.sort_by_key(|e| e.file_name());
                    for entry in entries {
                        let path = entry.path();
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = path.is_dir();
                        let label = if is_dir { format!("[{name}]") } else { name.clone() };
                        let response = ui.selectable_label(false, label);
                        if response.double_clicked() {
                            if is_dir {
                                self.file_explorer_path = path.to_string_lossy().to_string();
                            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                                if ext == "json" {
                                    let path_str = path.to_string_lossy();
                                    if let Ok(scene) = vetrace_engine::scene::loader::load_scene(&path_str) {
                                        engine.clear_scene();
                                        let _ = engine.load_scene(scene);
                                    } else if let Ok(prefab) = vetrace_engine::Prefab::load(&path_str) {
                                        engine.clear_scene();
                                        let _ = engine.load_scene(prefab.into_scene());
                                    }
                                }
                            }
                        }
                    }
                }
            });
        self.bottom_rect = Some(resp.response.rect);
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::EditorWindow for MainWindow {}
