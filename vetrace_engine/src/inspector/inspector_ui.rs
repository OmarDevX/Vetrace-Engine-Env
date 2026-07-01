use crate::inspector::Inspectable;

pub fn draw_component_ui(ui: &mut egui::Ui, component: &mut dyn Inspectable) {
    for (name, any_field, type_id) in component.exported_fields_mut() {
        if *type_id == std::any::TypeId::of::<f32>() {
            if let Some(v) = any_field.downcast_mut::<f32>() {
                ui.horizontal(|ui| {
                    ui.label(name);
                    ui.add(egui::Slider::new(v, 0.0..=100.0)); // or TextEdit::singleline
                });
            }
        } else if *type_id == std::any::TypeId::of::<bool>() {
            if let Some(v) = any_field.downcast_mut::<bool>() {
                ui.checkbox(v, name);
            }
        }
        // Add support for other types as needed
    }
}
