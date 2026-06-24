pub mod export;

use crate::components::components::ColliderShape;
use egui::Ui;
use export::{ExportKind, ExportedField};
use std::any::TypeId;

/// Trait implemented by user components via `#[derive(Inspectable)]`
pub trait Inspectable {
    fn exported_fields_mut(&mut self) -> Vec<ExportedField>;

    fn draw_ui(&mut self, ui: &mut Ui) {
        for field in self.exported_fields_mut() {
            unsafe {
                match field.kind {
                    ExportKind::Slider { min, max } => {
                        if field.type_id == TypeId::of::<f32>() {
                            let val = &mut *(field.value as *mut f32);
                            ui.add(egui::Slider::new(val, min..=max).text(field.name));
                        } else if field.type_id == TypeId::of::<f64>() {
                            let val = &mut *(field.value as *mut f64);
                            let mut val_f32 = *val as f32;
                            if ui
                                .add(egui::Slider::new(&mut val_f32, min..=max).text(field.name))
                                .changed()
                            {
                                *val = val_f32 as f64;
                            }
                        } else if field.type_id == TypeId::of::<i32>() {
                            let val = &mut *(field.value as *mut i32);
                            let mut val_f32 = *val as f32;
                            if ui
                                .add(egui::Slider::new(&mut val_f32, min..=max).text(field.name))
                                .changed()
                            {
                                *val = val_f32 as i32;
                            }
                        } else if field.type_id == TypeId::of::<u32>() {
                            let val = &mut *(field.value as *mut u32);
                            let mut val_f32 = *val as f32;
                            if ui
                                .add(egui::Slider::new(&mut val_f32, min..=max).text(field.name))
                                .changed()
                            {
                                *val = val_f32 as u32;
                            }
                        }
                    }

                    ExportKind::Drag { min, max, speed } => {
                        if field.type_id == TypeId::of::<f32>() {
                            let val = &mut *(field.value as *mut f32);
                            ui.add(
                                egui::DragValue::new(val)
                                    .clamp_range(min..=max)
                                    .speed(speed)
                                    .prefix(format!("{}: ", field.name)),
                            );
                        } else if field.type_id == TypeId::of::<f64>() {
                            let val = &mut *(field.value as *mut f64);
                            ui.add(
                                egui::DragValue::new(val)
                                    .clamp_range((min as f64)..=(max as f64))
                                    .speed(speed as f64)
                                    .prefix(format!("{}: ", field.name)),
                            );
                        } else if field.type_id == TypeId::of::<i32>() {
                            let val = &mut *(field.value as *mut i32);
                            ui.add(
                                egui::DragValue::new(val)
                                    .clamp_range((min as i32)..=(max as i32))
                                    .speed(speed)
                                    .prefix(format!("{}: ", field.name)),
                            );
                        } else if field.type_id == TypeId::of::<u32>() {
                            let val = &mut *(field.value as *mut u32);
                            ui.add(
                                egui::DragValue::new(val)
                                    .clamp_range((min as u32)..=(max as u32))
                                    .speed(speed)
                                    .prefix(format!("{}: ", field.name)),
                            );
                        }
                    }
                    ExportKind::Checkbox => {
                        if field.type_id == TypeId::of::<bool>() {
                            let val = &mut *(field.value as *mut bool);
                            ui.checkbox(val, field.name);
                        }
                    }
                    ExportKind::Text => {
                        if field.type_id == TypeId::of::<String>() {
                            let val = &mut *(field.value as *mut String);
                            ui.horizontal(|ui| {
                                ui.label(field.name);
                                ui.text_edit_singleline(val);
                            });
                        }
                    }
                    ExportKind::Dropdown(ref options) => {
                        if field.type_id == TypeId::of::<ColliderShape>() {
                            let val = &mut *(field.value as *mut ColliderShape);
                            let mut idx = *val as usize;
                            egui::ComboBox::from_label(field.name)
                                .selected_text(options[idx].clone())
                                .show_ui(ui, |ui| {
                                    for (i, opt) in options.iter().enumerate() {
                                        ui.selectable_value(&mut idx, i, opt);
                                    }
                                });
                            *val = match idx {
                                0 => ColliderShape::Sphere,
                                1 => ColliderShape::Cube,
                                2 => ColliderShape::Capsule,
                                _ => *val,
                            };
                        } else {
                            ui.label(format!("Dropdown for {}: {:?}", field.name, options));
                        }
                    }
                }
            }
        }
    }
}

/// Object-safe wrapper to allow runtime inspection of any Inspectable component
pub trait InspectableComponent: 'static {
    fn type_id(&self) -> TypeId;
    fn inspect(&mut self, ui: &mut Ui);
}

impl<T: Inspectable + 'static> InspectableComponent for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn inspect(&mut self, ui: &mut Ui) {
        self.draw_ui(ui);
    }
}
