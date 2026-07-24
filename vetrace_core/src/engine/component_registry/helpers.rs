use super::*;

pub(super) fn schema_field_mut<'a>(
    fields: &'a mut [FieldSchema],
    segments: &[FieldSegment],
) -> Option<&'a mut FieldSchema> {
    let (segment, remaining) = segments.split_first()?;
    match segment {
        FieldSegment::Field(name) => {
            let field = fields.iter_mut().find(|field| field.name == *name)?;
            if remaining.is_empty() { Some(field) } else { schema_field_mut(&mut field.children, remaining) }
        }
        FieldSegment::Index(_) => {
            let item = fields.iter_mut().find(|field| field.name == "item")?;
            if remaining.is_empty() { Some(item) } else { schema_field_mut(&mut item.children, remaining) }
        }
    }
}

pub(super) fn reflected_descriptor<T>(
    stable_id: &'static str,
    display_name: &'static str,
    category: &'static str,
) -> ComponentDescriptor
where
    T: Component + Clone + Default + Serialize + DeserializeOwned,
{
    let mut descriptor = ComponentDescriptor::new::<T>(stable_id, display_name).with_category(category);
    descriptor.serialize = Some(serialize_component::<T>);
    descriptor.deserialize = Some(deserialize_reflected_component::<T>);
    descriptor.clone_component = Some(clone_component::<T>);
    descriptor.create = Some(create_component::<T>);
    descriptor.persistent = true;
    descriptor.lua_accessible = true;
    descriptor
}

pub(super) fn short_type_name(type_name: &'static str) -> &'static str {
    type_name.rsplit("::").next().unwrap_or(type_name)
}

pub(super) fn normalize_alias(alias: &str) -> String {
    alias
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}


pub(super) fn serialize_component<T>(engine: &Engine, actor: Actor) -> Option<serde_json::Value>
where
    T: Component + Serialize,
{
    serde_json::to_value(actor.get_component::<T>(engine)?).ok()
}

pub(super) fn deserialize_component<T>(engine: &mut Engine, actor: Actor, value: serde_json::Value) -> Result<(), String>
where
    T: Component + DeserializeOwned,
{
    let component = serde_json::from_value::<T>(value).map_err(|error| error.to_string())?;
    actor.insert(engine, component).map_err(|error| error.to_string())
}

/// Deserialize a fully reflected component while accepting `{}` as the
/// conventional scripting/editor spelling for a unit marker component.
///
/// Serde represents a Rust unit struct as JSON `null`, while Lua naturally
/// turns an empty table into `{}`. Normalizing only when the component's Rust
/// default is unit/null keeps empty object values intact for real map-shaped
/// components.
pub(super) fn deserialize_reflected_component<T>(
    engine: &mut Engine,
    actor: Actor,
    value: serde_json::Value,
) -> Result<(), String>
where
    T: Component + Default + Serialize + DeserializeOwned,
{
    let default = DynamicValue::from_serialize(&T::default())
        .map_err(|error| error.to_string())?;
    let value = DynamicValue::from_json(value);
    let component = reflected_component_from_value::<T>(default, value)?;
    actor.insert(engine, component).map_err(|error| error.to_string())
}

pub(super) fn create_component<T>(engine: &mut Engine, actor: Actor, value: Option<DynamicValue>) -> Result<(), String>
where
    T: Component + Default + Serialize + DeserializeOwned,
{
    let component = match value {
        Some(value) => {
            let default = DynamicValue::from_serialize(&T::default())
                .map_err(|error| error.to_string())?;
            reflected_component_from_value::<T>(default, value)?
        }
        None => T::default(),
    };
    actor.insert(engine, component).map_err(|error| error.to_string())
}

pub(super) fn reflected_component_from_value<T>(
    mut default: DynamicValue,
    value: DynamicValue,
) -> Result<T, String>
where
    T: Default + DeserializeOwned,
{
    if matches!(default, DynamicValue::Null) && is_empty_marker_value(&value) {
        return Ok(T::default());
    }
    merge_dynamic_patch(&mut default, value);
    default.deserialize::<T>().map_err(|error| error.to_string())
}

pub(super) fn is_empty_marker_value(value: &DynamicValue) -> bool {
    matches!(value, DynamicValue::Null)
        || matches!(value, DynamicValue::Object(fields) if fields.is_empty())
}

pub(super) fn clone_component<T>(engine: &mut Engine, source: Actor, target: Actor) -> Result<(), String>
where
    T: Component + Clone,
{
    let Some(component) = source.get_component::<T>(engine).cloned() else { return Ok(()); };
    target.insert(engine, component).map_err(|error| error.to_string())
}

pub(super) fn remove_component<T: Component>(engine: &mut Engine, actor: Actor) -> bool {
    actor.remove::<T>(engine).is_some()
}
