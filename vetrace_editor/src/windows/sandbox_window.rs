use vetrace_engine::{engine::engine::Engine, scene::object::Object};
use egui::{Context, Slider, TextEdit, Ui};

/// Simple sandbox window for spawning objects and tweaking basic settings.
#[derive(Clone)]
pub struct SandboxWindow {
    pub new_object: Object,
    pub skycolor: [f32; 3],
    pub is_fisheye: bool,
    pub new_object_position_str: [String; 3],
    pub new_object_size_str: [String; 3],
}

impl SandboxWindow {
    pub fn new() -> Self {
        Self {
            new_object: Object::default(),
            is_fisheye: false,
            skycolor: [30.0, 255.0, 255.0],
            new_object_position_str: ["0.0".to_owned(), "0.0".to_owned(), "0.0".to_owned()],
            new_object_size_str: ["1.0".to_owned(), "1.0".to_owned(), "1.0".to_owned()],
        }
    }

    pub fn ui(&mut self, _ctx: &Context, ui: &mut Ui, engine: &mut Engine) {
        ui.heading("Sandbox Window");
        ui.separator();

        ui.checkbox(&mut self.is_fisheye, "Enable Fisheye");

        ui.horizontal(|ui| {
            ui.label("Sky Color");
            for i in 0..3 {
                ui.add(Slider::new(&mut self.skycolor[i], 0.0..=255.0).text(["R", "G", "B"][i]));
            }
        });

        ui.collapsing("Add New Object", |ui| {
            for i in 0..3 {
                ui.horizontal(|ui| {
                    ui.label(["Position X", "Position Y", "Position Z"][i]);
                    ui.add(TextEdit::singleline(&mut self.new_object_position_str[i]).desired_width(60.0));
                });
            }

            if self.new_object.is_cube {
                for i in 0..3 {
                    ui.horizontal(|ui| {
                        ui.label(["Size X", "Size Y", "Size Z"][i]);
                        ui.add(TextEdit::singleline(&mut self.new_object_size_str[i]).desired_width(60.0));
                    });
                }
            }

            ui.add(Slider::new(&mut self.new_object.radius, 0.1..=100.0).text("Radius"));
            ui.checkbox(&mut self.new_object.is_cube, "Is Cube");

            if ui.button("Add Object").clicked() {
                let mut new_object = self.new_object.clone();
                for i in 0..3 {
                    new_object.position[i] = self.new_object_position_str[i].parse::<f32>().unwrap_or(0.0);
                    if new_object.is_cube {
                        new_object.size[i] = self.new_object_size_str[i]
                            .parse::<f32>()
                            .unwrap_or(1.0)
                            .max(0.1);
                    }
                }
                engine.spawn_object(new_object);
            }
        });
    }
}

impl Default for SandboxWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::EditorWindow for SandboxWindow {}
