use egui::{self, Area, RichText, FontId, FontFamily};
use crate::components::components::{
    UILabel, UILayout, UIScreenSpace, Anchor, UIPanel, UIButton, UITextEditor, UIList,
};
use mlua::Value as LuaValue;
use super::Engine;
// Note: MainWindow and SandboxWindow have been moved to vetrace_editor crate
// GameUIRenderer is defined in the game_ui module

/// Trait for plugins that can render editor UI
pub trait EditorUIRenderer {
    /// Render the editor UI for this plugin
    fn render_editor_ui(&mut self, ctx: &egui::Context, engine: &mut Engine) -> Result<(), Box<dyn std::error::Error>>;
}

impl Engine {
    pub fn draw_game_ui(&mut self, ctx: &egui::Context) {
        let (w, h) = self.window.get_size();
        let engine_ptr = self as *mut Engine;
        for (e, panel, layout, _) in self
            .world
            .query3_mut::<UIPanel, UILayout, UIScreenSpace>()
        {
            let (mut x, mut y) = match layout.anchor {
                Anchor::TopLeft => (0.0, 0.0),
                Anchor::TopCenter => (w as f32 / 2.0, 0.0),
                Anchor::TopRight => (w as f32, 0.0),
                Anchor::CenterLeft => (0.0, h as f32 / 2.0),
                Anchor::Center => (w as f32 / 2.0, h as f32 / 2.0),
                Anchor::CenterRight => (w as f32, h as f32 / 2.0),
                Anchor::BottomLeft => (0.0, h as f32),
                Anchor::BottomCenter => (w as f32 / 2.0, h as f32),
                Anchor::BottomRight => (w as f32, h as f32),
            };
            x += layout.offset[0];
            y += layout.offset[1];
            let color = egui::Color32::from_rgba_unmultiplied(
                panel.color[0] as u8,
                panel.color[1] as u8,
                panel.color[2] as u8,
                panel.color[3] as u8,
            );
            Area::new(format!("ui_panel_{}", e.0).into())
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(color)
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(panel.size[0], panel.size[1]));
                        });
                });
        }

        for (e, button, layout, _) in self
            .world
            .query3_mut::<UIButton, UILayout, UIScreenSpace>()
        {
            let (mut x, mut y) = match layout.anchor {
                Anchor::TopLeft => (0.0, 0.0),
                Anchor::TopCenter => (w as f32 / 2.0, 0.0),
                Anchor::TopRight => (w as f32, 0.0),
                Anchor::CenterLeft => (0.0, h as f32 / 2.0),
                Anchor::Center => (w as f32 / 2.0, h as f32 / 2.0),
                Anchor::CenterRight => (w as f32, h as f32 / 2.0),
                Anchor::BottomLeft => (0.0, h as f32),
                Anchor::BottomCenter => (w as f32 / 2.0, h as f32),
                Anchor::BottomRight => (w as f32, h as f32),
            };
            x += layout.offset[0];
            y += layout.offset[1];
            Area::new(format!("ui_button_{}", e.0).into())
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    let rich = RichText::new(&button.text).color(egui::Color32::from_rgba_unmultiplied(
                        button.text_color[0] as u8,
                        button.text_color[1] as u8,
                        button.text_color[2] as u8,
                        button.text_color[3] as u8,
                    ));
                    let resp = ui
                        .add(
                            egui::Button::new(rich)
                                .min_size(egui::vec2(button.size[0], button.size[1]))
                                .fill(egui::Color32::from_rgba_unmultiplied(
                                    button.bg_color[0] as u8,
                                    button.bg_color[1] as u8,
                                    button.bg_color[2] as u8,
                                    button.bg_color[3] as u8,
                                )),
                        );
                    button.clicked = resp.clicked();
                    button.hovered = resp.hovered();
                    button.pressed = resp.is_pointer_button_down_on();
                    if resp.clicked() {
                        unsafe { (&mut *engine_ptr).emit_signal(e, "clicked", LuaValue::Nil); }
                    }
                });
        }

        for (e, editor, layout, _) in self
            .world
            .query3_mut::<UITextEditor, UILayout, UIScreenSpace>()
        {
            let (mut x, mut y) = match layout.anchor {
                Anchor::TopLeft => (0.0, 0.0),
                Anchor::TopCenter => (w as f32 / 2.0, 0.0),
                Anchor::TopRight => (w as f32, 0.0),
                Anchor::CenterLeft => (0.0, h as f32 / 2.0),
                Anchor::Center => (w as f32 / 2.0, h as f32 / 2.0),
                Anchor::CenterRight => (w as f32, h as f32 / 2.0),
                Anchor::BottomLeft => (0.0, h as f32),
                Anchor::BottomCenter => (w as f32 / 2.0, h as f32),
                Anchor::BottomRight => (w as f32, h as f32),
            };
            x += layout.offset[0];
            y += layout.offset[1];
            Area::new(format!("ui_edit_{}", e.0).into())
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    let resp = ui.add_sized(
                        egui::vec2(editor.size[0], editor.size[1]),
                        egui::TextEdit::singleline(&mut editor.text),
                    );
                    editor.changed = resp.changed();
                    editor.hovered = resp.hovered();
                    editor.focused = resp.has_focus();
                    if resp.changed() {
                        unsafe { (&mut *engine_ptr).emit_signal_string(e, "changed", &editor.text); }
                    }
                });
        }
        for (e, list, layout, _) in self
            .world
            .query3_mut::<UIList, UILayout, UIScreenSpace>()
        {
            let (mut x, mut y) = match layout.anchor {
                Anchor::TopLeft => (0.0, 0.0),
                Anchor::TopCenter => (w as f32 / 2.0, 0.0),
                Anchor::TopRight => (w as f32, 0.0),
                Anchor::CenterLeft => (0.0, h as f32 / 2.0),
                Anchor::Center => (w as f32 / 2.0, h as f32 / 2.0),
                Anchor::CenterRight => (w as f32, h as f32 / 2.0),
                Anchor::BottomLeft => (0.0, h as f32),
                Anchor::BottomCenter => (w as f32 / 2.0, h as f32),
                Anchor::BottomRight => (w as f32, h as f32),
            };
            x += layout.offset[0];
            y += layout.offset[1];
            Area::new(format!("ui_list_{}", e.0).into())
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .rounding(8.0)
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(list.size[0], list.size[1]));
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for item in &list.items {
                                    ui.label(
                                        egui::RichText::new(item).color(
                                            egui::Color32::from_rgba_unmultiplied(
                                                list.color[0] as u8,
                                                list.color[1] as u8,
                                                list.color[2] as u8,
                                                list.color[3] as u8,
                                            ),
                                        ),
                                    );
                                }
                            });
                        });
                });
        }
        for (e, label, layout, _) in self.world.query3_mut::<crate::components::components::UILabel, crate::components::components::UILayout, crate::components::components::UIScreenSpace>() {
            let (mut x, mut y) = match layout.anchor {
                crate::components::components::Anchor::TopLeft => (0.0, 0.0),
                crate::components::components::Anchor::TopCenter => (w as f32 / 2.0, 0.0),
                crate::components::components::Anchor::TopRight => (w as f32, 0.0),
                crate::components::components::Anchor::CenterLeft => (0.0, h as f32 / 2.0),
                crate::components::components::Anchor::Center => (w as f32 / 2.0, h as f32 / 2.0),
                crate::components::components::Anchor::CenterRight => (w as f32, h as f32 / 2.0),
                crate::components::components::Anchor::BottomLeft => (0.0, h as f32),
                crate::components::components::Anchor::BottomCenter => (w as f32 / 2.0, h as f32),
                crate::components::components::Anchor::BottomRight => (w as f32, h as f32),
            };
            x += layout.offset[0];
            y += layout.offset[1];
            let color = egui::Color32::from_rgba_unmultiplied(
                label.color[0] as u8,
                label.color[1] as u8,
                label.color[2] as u8,
                label.color[3] as u8,
            );
            egui::Area::new(format!("ui_label_{}", e.0).into())
                .fixed_pos(egui::pos2(x, y))
                .show(ctx, |ui| {
                    let mut text = egui::RichText::new(&label.text)
                        .size(label.font_size)
                        .color(color);
                    if let Some(name) = &label.font_name {
                        let family = if name.to_lowercase() == "monospace" { egui::FontFamily::Monospace } else { egui::FontFamily::Proportional };
                        text = text.font(egui::FontId::new(label.font_size, family));
                    }
                    ui.label(text);
                });
        }
    }

    /// Draw editor UI - now handled by vetrace_editor plugin
    /// This method is kept for compatibility but does nothing
    pub fn draw_editor_ui(&mut self, ctx: &egui::Context) {
        // Show a simple test window to verify EGUI is working
        egui::Window::new("Engine UI Test")
            .default_open(true)
            .show(ctx, |ui| {
                ui.label("🎮 Vetrace Engine - EGUI Test");
                ui.separator();
                ui.label("✅ EGUI is working and interactive!");
                ui.label("Editor plugin should render its UI here.");

                if ui.button("Test Button").clicked() {
                    println!("🎉 Button clicked! EGUI interaction confirmed!");
                }

                ui.label("Try clicking the button above to test interaction.");
            });

        // Call all registered UI callbacks
        let engine_ptr = self as *mut Engine;
        for callback in self.ui_callbacks.iter_mut() {
            // SAFETY: we only create a temporary mutable reference for the callback
            let engine: &mut Engine = unsafe { &mut *engine_ptr };
            if let Err(e) = callback(ctx, engine) {
                println!("⚠️ Error in UI callback: {}", e);
            }
        }
    }

    /// Draw editor UI with plugin manager access
    /// This allows the plugin manager to render plugin UIs
    pub fn draw_editor_ui_with_plugins(&mut self, ctx: &egui::Context, plugin_manager: &mut crate::app::plugin::PluginManager) {
        // Show the basic engine test window
        self.draw_editor_ui(ctx);

        // Render plugin UIs
        if let Err(e) = plugin_manager.render_plugin_uis(ctx, self) {
            println!("⚠️ Error rendering plugin UIs: {}", e);
        }
    }

    /// Register a UI callback
    pub fn add_ui_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&egui::Context, &mut Engine) -> Result<(), Box<dyn std::error::Error>> + 'static,
    {
        self.ui_callbacks.push(Box::new(callback));
    }

    /// Clear all registered UI callbacks
    pub fn clear_ui_callbacks(&mut self) {
        self.ui_callbacks.clear();
    }

    /// Draw editor UI with access to an editor plugin
    /// This method allows the engine to render editor plugin UI
    pub fn draw_editor_ui_with_plugin<T>(&mut self, ctx: &egui::Context, editor_plugin: Option<&mut T>)
    where
        T: EditorUIRenderer,
    {
        // Show the basic engine test window
        self.draw_editor_ui(ctx);

        // If we have an editor plugin, render its UI
        if let Some(plugin) = editor_plugin {
            if let Err(e) = plugin.render_editor_ui(ctx, self) {
                println!("⚠️ Error rendering editor UI: {}", e);
            }
        }
    }
}