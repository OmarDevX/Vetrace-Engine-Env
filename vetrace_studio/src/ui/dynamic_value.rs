use super::*;

pub(super) fn draw_dynamic_value(
    ui: &mut egui::Ui,
    component_id: &str,
    schema: &FieldSchema,
    path: &FieldPath,
    value: &mut DynamicValue,
) -> bool {
    if !schema.editable {
        ui.horizontal(|ui| {
            ui.label(&schema.display_name);
            ui.label(format_dynamic(value));
        });
        return false;
    }

    match value {
        DynamicValue::Bool(current) => ui.checkbox(current, &schema.display_name).changed(),
        DynamicValue::I64(current) => {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                ui.add(egui::DragValue::new(current)).changed()
            }).inner
        }
        DynamicValue::U64(current) => {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                ui.add(egui::DragValue::new(current)).changed()
            }).inner
        }
        DynamicValue::F64(current) => {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                let mut drag = egui::DragValue::new(current)
                    .speed(schema.numeric_range.as_ref().and_then(|range| range.step).unwrap_or(0.1));
                if let Some(range) = &schema.numeric_range {
                    drag = drag.range(
                        range.min.unwrap_or(f64::MIN)..=range.max.unwrap_or(f64::MAX),
                    );
                }
                ui.add(drag).changed()
            }).inner
        }
        DynamicValue::String(current) if schema.kind == FieldKind::Enum => {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                if schema.enum_variants.is_empty() {
                    ui.add_enabled(false, egui::Label::new(format!(
                        "{} (enum options not registered)",
                        current
                    )));
                    false
                } else {
                    let before = current.clone();
                    egui::ComboBox::from_id_source(("enum_field", component_id, path.to_string()))
                        .selected_text(humanize_enum_variant(current))
                        .show_ui(ui, |ui| {
                            for variant in &schema.enum_variants {
                                ui.selectable_value(
                                    current,
                                    variant.clone(),
                                    humanize_enum_variant(variant),
                                );
                            }
                        });
                    *current != before
                }
            }).inner
        }
        DynamicValue::String(current) => {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                ui.text_edit_singleline(current).changed()
            }).inner
        }
        DynamicValue::Array(values)
            if matches!(schema.kind, FieldKind::Vec2 | FieldKind::Vec3 | FieldKind::Vec4 | FieldKind::Quaternion | FieldKind::Color) =>
        {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                let mut changed = false;
                for (index, value) in values.iter_mut().enumerate() {
                    ui.label(["X", "Y", "Z", "W"].get(index).copied().unwrap_or("#"));
                    changed |= numeric_drag(ui, value);
                }
                changed
            }).inner
        }
        DynamicValue::Object(values) => {
            let mut changed = false;
            egui::CollapsingHeader::new(&schema.display_name)
                .id_source(("field", component_id, path.to_string()))
                .default_open(true)
                .show(ui, |ui| {
                    for child in &schema.children {
                        if let Some(child_value) = values.get_mut(&child.name) {
                            let child_path = path.clone().field(child.name.clone());
                            changed |= draw_dynamic_value(ui, component_id, child, &child_path, child_value);
                        }
                    }
                });
            changed
        }
        DynamicValue::Array(values) => {
            let mut changed = false;
            egui::CollapsingHeader::new(format!("{} [{}]", schema.display_name, values.len()))
                .id_source(("array", component_id, path.to_string()))
                .show(ui, |ui| {
                    let item_schema = schema.children.iter().find(|child| child.name == "item");
                    for (index, item) in values.iter_mut().enumerate() {
                        let fallback;
                        let item_schema = match item_schema {
                            Some(schema) => schema,
                            None => {
                                fallback = FieldSchema::from_dynamic(
                                    "item",
                                    format!("Item {index}"),
                                    item.clone(),
                                );
                                &fallback
                            }
                        };
                        let mut display_schema = item_schema.clone();
                        display_schema.display_name = format!("Item {index}");
                        changed |= draw_dynamic_value(
                            ui,
                            component_id,
                            &display_schema,
                            &path.clone().index(index),
                            item,
                        );
                    }
                });
            changed
        }
        DynamicValue::Null => {
            ui.horizontal(|ui| {
                ui.label(&schema.display_name);
                ui.label("null");
            });
            false
        }
    }
}

pub(super) fn humanize_enum_variant(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lowercase = false;
    for character in value.chars() {
        if character == '_' || character == '-' {
            if !output.ends_with(' ') { output.push(' '); }
            previous_lowercase = false;
            continue;
        }
        if character.is_uppercase() && previous_lowercase { output.push(' '); }
        if output.is_empty() {
            output.extend(character.to_uppercase());
        } else {
            output.push(character);
        }
        previous_lowercase = character.is_lowercase();
    }
    output
}

pub(super) fn numeric_drag(ui: &mut egui::Ui, value: &mut DynamicValue) -> bool {
    match value {
        DynamicValue::F64(value) => ui.add(egui::DragValue::new(value).speed(0.05)).changed(),
        DynamicValue::I64(value) => ui.add(egui::DragValue::new(value)).changed(),
        DynamicValue::U64(value) => ui.add(egui::DragValue::new(value)).changed(),
        _ => {
            ui.label(format_dynamic(value));
            false
        }
    }
}

pub(super) fn format_dynamic(value: &DynamicValue) -> String {
    match value {
        DynamicValue::Null => "null".into(),
        DynamicValue::Bool(value) => value.to_string(),
        DynamicValue::I64(value) => value.to_string(),
        DynamicValue::U64(value) => value.to_string(),
        DynamicValue::F64(value) => format!("{value:.3}"),
        DynamicValue::String(value) => value.clone(),
        DynamicValue::Array(values) => format!("[{} values]", values.len()),
        DynamicValue::Object(values) => format!("{{{} fields}}", values.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::humanize_enum_variant;

    #[test]
    fn enum_labels_are_human_readable_without_changing_serialized_values() {
        assert_eq!(humanize_enum_variant("hybrid_realtime_direct"), "Hybrid realtime direct");
        assert_eq!(humanize_enum_variant("LessEqual"), "Less Equal");
        assert_eq!(humanize_enum_variant("SCREAMING-KEBAB"), "SCREAMING KEBAB");
    }
}
