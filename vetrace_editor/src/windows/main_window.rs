//! Main Editor Window Implementation
//! 
//! This is the primary editor interface, moved from the main engine
//! to keep editor functionality separate.

use vetrace_engine::{ecs::Entity, engine::engine::Engine, scene::object::Object};
use egui::{
    ComboBox, Context, Margin, ScrollArea, Slider, TextEdit, TopBottomPanel, Ui, Window,
};
use rfd::FileDialog;
use std::collections::HashMap;
use transform_gizmo_egui::GizmoOrientation;

use super::{NewField, EditorGizmoMode, SandboxWindow};
use crate::EditorWindow;

/// Main editor window containing all the primary editor panels
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
    /// Create a new main window
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
    
    /// Main UI rendering function
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
    
    /// Get blur rectangles for background effects
    pub fn blur_rects(&self) -> Vec<egui::Rect> {
        let mut r = Vec::new();
        if let Some(rect) = self.left_rect {
            r.push(rect);
        }
        if let Some(rect) = self.right_rect {
            r.push(rect);
        }
        if let Some(rect) = self.bottom_rect {
            r.push(rect);
        }
        if let Some(rect) = self.top_rect {
            r.push(rect);
        }
        r
    }
    
    /// Top panel with main controls
    fn top_panel_ui(&mut self, ctx: &Context, engine: &mut Engine) {
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
                    // Gizmo mode selector
                    ComboBox::from_id_source("gizmo_mode_combo")
                        .selected_text(match self.gizmo_mode {
                            EditorGizmoMode::Translate => "Move",
                            EditorGizmoMode::Rotate => "Rotate",
                            EditorGizmoMode::Scale => "Scale",
                            EditorGizmoMode::Omni => "Omni",
                            EditorGizmoMode::Arcball => "Arcball",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                EditorGizmoMode::Translate,
                                "Move",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                EditorGizmoMode::Rotate,
                                "Rotate",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                EditorGizmoMode::Scale,
                                "Scale",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                EditorGizmoMode::Omni,
                                "Omni",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_mode,
                                EditorGizmoMode::Arcball,
                                "Arcball",
                            );
                        });

                    ui.separator();

                    // Gizmo orientation selector
                    ComboBox::from_id_source("gizmo_orientation_combo")
                        .selected_text(match self.gizmo_orientation {
                            GizmoOrientation::Global => "Global",
                            GizmoOrientation::Local => "Local",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.gizmo_orientation,
                                GizmoOrientation::Global,
                                "Global",
                            );
                            ui.selectable_value(
                                &mut self.gizmo_orientation,
                                GizmoOrientation::Local,
                                "Local",
                            );
                        });

                    ui.separator();

                    // File operations
                    if ui.button("Save Scene").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .save_file()
                        {
                            let path_str = path.to_string_lossy();
                            let _ = engine.save_scene_to_file(&path_str);
                        }
                    }

                    if ui.button("Load Scene").clicked() {
                        if let Some(path) = FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            let path_str = path.to_string_lossy();
                            if let Ok(scene) = vetrace_engine::scene::loader::load_scene(&path_str) {
                                engine.clear_scene();
                                let _ = engine.load_scene(scene);
                            }
                        }
                    }
                });
            });
        self.top_rect = Some(resp.response.rect);
    }
    
    /// Left panel with scene hierarchy and controls
    fn left_panel_ui(&mut self, ctx: &Context, engine: &mut Engine) {
        let resp = egui::SidePanel::left("left_panel")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 60, 210))
                    .rounding(8.0)
                    .inner_margin(Margin::same(6.0))
                    .outer_margin(Margin {
                        left: 10.0,
                        right: 0.0,
                        top: 10.0,
                        bottom: 10.0,
                    }),
            )
            .resizable(true)
            .max_width(350.0)
            .min_width(150.0)
            .default_width(250.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("✢ Vetrace Engine");
                    });
                    
                    // Engine controls
                    ui.horizontal(|ui| {
                        if engine.paused {
                            if ui.button("Start").clicked() {
                                engine.resume();
                            }
                        } else {
                            if ui.button("Stop").clicked() {
                                engine.pause();
                            }
                        }
                        if ui.button("Restart").clicked() {
                            engine.restart();
                        }
                    });
                    ui.separator();

                    // Window toggles
                    if ui.button("Toggle Sandbox Window").clicked() {
                        self.show_sandbox_window = !self.show_sandbox_window;
                    }

                    ui.separator();

                    // Scene hierarchy
                    ui.heading("Scene Hierarchy");
                    self.draw_scene_hierarchy(ui, engine);

                    ui.separator();
                    
                    // Links
                    use egui::special_emojis::GITHUB;
                    ui.hyperlink_to(
                        format!("{GITHUB} Source Code"),
                        "https://github.com/OmarDevX",
                    );
                });
            });
        self.left_rect = Some(resp.response.rect);
    }
    
    /// Draw the scene hierarchy
    fn draw_scene_hierarchy(&mut self, ui: &mut Ui, engine: &mut Engine) {
        // This will be implemented to show the entity hierarchy
        ui.label("Entities:");

        // For now, just show selected entities count
        ui.label(format!("Selected: {}", self.selected_entities.len()));

        if ui.button("Clear Selection").clicked() {
            self.selected_entities.clear();
        }
    }

    /// Right panel with component inspector
    fn right_panel_ui(&mut self, ctx: &Context, engine: &mut Engine) {
        let resp = egui::SidePanel::right("right_panel")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgba_unmultiplied(40, 0, 60, 210))
                    .rounding(8.0)
                    .inner_margin(Margin::same(6.0))
                    .outer_margin(Margin {
                        left: 0.0,
                        right: 10.0,
                        top: 10.0,
                        bottom: 10.0,
                    }),
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

                        // Component editors
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

                        // Component adder
                        let mut names: Vec<_> = engine.component_adders.keys().cloned().collect();
                        for g in &engine.generated_components {
                            if !names.contains(g) {
                                names.push(g.clone());
                            }
                        }

                        ComboBox::from_id_source("component_selector")
                            .selected_text(&self.selected_component)
                            .show_ui(ui, |ui| {
                                for name in &names {
                                    ui.selectable_value(&mut self.selected_component, name.clone(), name);
                                }
                            });

                        if ui.button("Add Component").clicked() && !self.selected_component.is_empty() {
                            if let Some(adder) = engine.component_adders.get(&self.selected_component).cloned() {
                                adder(engine, entity);
                            } else if engine.generated_components.contains(&self.selected_component) {
                                engine.add_generated_component(entity, &self.selected_component);
                            }
                        }
                    } else if self.selected_entities.len() > 1 {
                        ui.label(format!("Multiple entities selected ({})", self.selected_entities.len()));
                        ui.label("Multi-edit not yet supported");
                    } else {
                        ui.label("No entity selected");
                        ui.label("Click on an object in the scene to select it");
                    }
                });
            });
        self.right_rect = Some(resp.response.rect);
    }

    /// Bottom panel with file explorer
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

                ScrollArea::vertical().show(ui, |ui| {
                    if let Ok(entries) = fs::read_dir(&self.file_explorer_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            let is_dir = path.is_dir();
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            let label = if is_dir { format!("📁 {}", name) } else { format!("📄 {}", name) };

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
                                            let _ = engine.instantiate_prefab(prefab);
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
            });
        self.bottom_rect = Some(resp.response.rect);
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorWindow for MainWindow {
    fn initialize(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing Main Window...");
        Ok(())
    }
    
    fn update(&mut self, _engine: &mut Engine, _delta_time: f32) -> Result<(), Box<dyn std::error::Error>> {
        // Update logic for the main window
        Ok(())
    }
    
    fn render(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        // This is handled by the ui() method
        Ok(())
    }
    
    fn cleanup(&mut self, _engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>> {
        println!("Cleaning up Main Window...");
        Ok(())
    }
}
