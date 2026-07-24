use std::collections::BTreeMap;

use crate::reflection::{
    ComponentSchema, DynamicValue, FieldPath, FieldSchema, ReflectionError,
};
use crate::{Actor, Engine};

use super::component_registry::ComponentManager;


mod engine_api;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UpdateAccess {
    Generic,
    Lua,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldVisibility {
    Serializable,
    Lua,
}

fn component_default_value(schema: &ComponentSchema) -> DynamicValue {
    if schema.fields.len() == 1 && schema.fields[0].name == "value" {
        return schema.fields[0].default_value.clone();
    }
    DynamicValue::Object(
        schema
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.default_value.clone()))
            .collect(),
    )
}

fn field_visible(field: &FieldSchema, visibility: FieldVisibility) -> bool {
    match visibility {
        FieldVisibility::Serializable => field.serializable,
        FieldVisibility::Lua => field.lua_accessible,
    }
}

fn filter_component_value(
    value: &DynamicValue,
    schema: &ComponentSchema,
    visibility: FieldVisibility,
) -> Option<DynamicValue> {
    if schema.fields.len() == 1 && schema.fields[0].name == "value" {
        return filter_field_value(value, &schema.fields[0], visibility);
    }
    let DynamicValue::Object(values) = value else { return Some(value.clone()); };
    let mut filtered = BTreeMap::new();
    for field in &schema.fields {
        if !field_visible(field, visibility) { continue; }
        let Some(value) = values.get(&field.name) else { continue; };
        if let Some(value) = filter_field_value(value, field, visibility) {
            filtered.insert(field.name.clone(), value);
        }
    }
    Some(DynamicValue::Object(filtered))
}

fn filter_field_value(
    value: &DynamicValue,
    field: &FieldSchema,
    visibility: FieldVisibility,
) -> Option<DynamicValue> {
    if !field_visible(field, visibility) { return None; }
    if field.children.is_empty() { return Some(value.clone()); }
    match value {
        DynamicValue::Object(values) => {
            let mut filtered = BTreeMap::new();
            for child in &field.children {
                if !field_visible(child, visibility) { continue; }
                let Some(value) = values.get(&child.name) else { continue; };
                if let Some(value) = filter_field_value(value, child, visibility) {
                    filtered.insert(child.name.clone(), value);
                }
            }
            Some(DynamicValue::Object(filtered))
        }
        DynamicValue::Array(values) => {
            let item = field.children.iter().find(|child| child.name == "item")?;
            Some(DynamicValue::Array(
                values
                    .iter()
                    .filter_map(|value| filter_field_value(value, item, visibility))
                    .collect(),
            ))
        }
        _ => Some(value.clone()),
    }
}

fn ensure_field_access(
    field: &FieldSchema,
    stable_id: &str,
    path: &FieldPath,
    access: UpdateAccess,
    require_editable: bool,
) -> Result<(), ReflectionError> {
    if matches!(access, UpdateAccess::Lua) && !field.lua_accessible {
        return Err(ReflectionError::LuaAccessDenied(format!("{stable_id}.{path}")));
    }
    if require_editable && !field.editable {
        return Err(ReflectionError::ComponentReadOnly(format!("{stable_id}.{path}")));
    }
    Ok(())
}

fn merge_component_update(
    target: &mut DynamicValue,
    patch: DynamicValue,
    schema: &ComponentSchema,
    access: UpdateAccess,
) -> Result<(), ReflectionError> {
    if schema.fields.len() == 1 && schema.fields[0].name == "value" {
        let path = FieldPath::root();
        ensure_field_access(&schema.fields[0], &schema.stable_id, &path, access, true)?;
        return merge_field_update(target, patch, &schema.fields[0], &schema.stable_id, &path, access);
    }
    let target_actual = target.type_name();
    let DynamicValue::Object(target_values) = target else {
        return Err(ReflectionError::TypeMismatch {
            path: String::new(),
            expected: "object",
            actual: target_actual,
        });
    };
    let patch_actual = patch.type_name();
    let DynamicValue::Object(patch_values) = patch else {
        return Err(ReflectionError::TypeMismatch {
            path: String::new(),
            expected: "object",
            actual: patch_actual,
        });
    };
    for (name, patch_value) in patch_values {
        let field = schema
            .fields
            .iter()
            .find(|field| field.name == name)
            .ok_or_else(|| ReflectionError::MissingField(name.clone()))?;
        let path = FieldPath::root().field(name.clone());
        ensure_field_access(field, &schema.stable_id, &path, access, true)?;
        let target_value = target_values
            .get_mut(&name)
            .ok_or_else(|| ReflectionError::MissingField(name.clone()))?;
        merge_field_update(target_value, patch_value, field, &schema.stable_id, &path, access)?;
    }
    Ok(())
}

fn merge_component_field_update(
    component: &mut DynamicValue,
    path: &FieldPath,
    patch: DynamicValue,
    schema: &ComponentSchema,
    access: UpdateAccess,
) -> Result<(), ReflectionError> {
    if path.is_root() {
        return merge_component_update(component, patch, schema, access);
    }
    let field = schema_field(schema, path)
        .ok_or_else(|| ReflectionError::MissingField(path.to_string()))?;
    ensure_field_access(field, &schema.stable_id, path, access, true)?;
    let target = component.get_mut(path)?;
    merge_field_update(target, patch, field, &schema.stable_id, path, access)
}

fn merge_field_update(
    target: &mut DynamicValue,
    patch: DynamicValue,
    field: &FieldSchema,
    stable_id: &str,
    path: &FieldPath,
    access: UpdateAccess,
) -> Result<(), ReflectionError> {
    if field.children.is_empty() {
        *target = patch;
        return Ok(());
    }
    match (target, patch) {
        (DynamicValue::Object(target_values), DynamicValue::Object(patch_values)) => {
            for (name, patch_value) in patch_values {
                let child = field
                    .children
                    .iter()
                    .find(|child| child.name == name)
                    .ok_or_else(|| ReflectionError::MissingField(format!("{path}.{name}")))?;
                let child_path = FieldPath::new(path.segments().to_vec()).field(name.clone());
                ensure_field_access(child, stable_id, &child_path, access, true)?;
                let target_value = target_values
                    .get_mut(&name)
                    .ok_or_else(|| ReflectionError::MissingField(child_path.to_string()))?;
                merge_field_update(target_value, patch_value, child, stable_id, &child_path, access)?;
            }
            Ok(())
        }
        (DynamicValue::Array(target_values), DynamicValue::Array(patch_values)) => {
            let Some(item) = field.children.iter().find(|child| child.name == "item") else {
                *target_values = patch_values;
                return Ok(());
            };
            ensure_field_access(item, stable_id, path, access, true)?;
            if item.children.is_empty() {
                *target_values = patch_values;
                return Ok(());
            }
            let previous = std::mem::take(target_values);
            let mut merged = Vec::with_capacity(patch_values.len());
            for (index, patch_value) in patch_values.into_iter().enumerate() {
                let mut target_value = previous
                    .get(index)
                    .cloned()
                    .unwrap_or_else(|| item.default_value.clone());
                let item_path = FieldPath::new(path.segments().to_vec()).index(index);
                merge_field_update(
                    &mut target_value,
                    patch_value,
                    item,
                    stable_id,
                    &item_path,
                    access,
                )?;
                merged.push(target_value);
            }
            *target_values = merged;
            Ok(())
        }
        (target, patch) => {
            *target = patch;
            Ok(())
        }
    }
}

fn schema_field<'a>(schema: &'a ComponentSchema, path: &FieldPath) -> Option<&'a FieldSchema> {
    let mut fields = schema.fields.as_slice();
    let mut current = None;
    for segment in path.segments() {
        match segment {
            crate::reflection::FieldSegment::Field(name) => {
                current = fields.iter().find(|field| field.name == *name);
                fields = current?.children.as_slice();
            }
            crate::reflection::FieldSegment::Index(_) => {
                current = fields.iter().find(|field| field.name == "item").or(current);
                fields = current?.children.as_slice();
            }
        }
    }
    current
}
