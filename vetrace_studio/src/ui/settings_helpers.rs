use super::*;

pub(super) fn settings_text(ui: &mut egui::Ui, label: &str, value: &mut String) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).desired_width(360.0)).changed()
    }).inner
}

pub(super) fn binding_list(ui: &mut egui::Ui, label: &str, values: &mut Vec<String>) {
    let mut text = values.join(", ");
    if settings_text(ui, label, &mut text) {
        *values = text.split(',').map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned).collect();
    }
}

pub(super) fn empty_project_path(value: &str) -> Option<ProjectPath> {
    let value = value.trim();
    if value.is_empty() { None } else { ProjectPath::new(value).ok() }
}

pub(super) fn enum_combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    id: impl std::hash::Hash,
    value: &mut T,
    choices: &[(T, &str)],
) {
    ui.horizontal(|ui| {
        ui.label(label);
        let selected = choices.iter().find(|(candidate, _)| candidate == value).map(|(_, label)| *label).unwrap_or("Unknown");
        egui::ComboBox::from_id_source(id).selected_text(selected).show_ui(ui, |ui| {
            for (candidate, label) in choices {
                ui.selectable_value(value, *candidate, *label);
            }
        });
    });
}
