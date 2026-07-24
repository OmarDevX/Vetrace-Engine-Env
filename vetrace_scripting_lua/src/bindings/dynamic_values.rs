use super::*;

pub(super) fn read_component_value(
    target: LuaEntityTarget,
    component: &str,
    path: &FieldPath,
) -> mlua::Result<DynamicValue> {
    with_live_actor(target, |engine, actor| {
        let stable_id = engine.ensure_lua_component_access(component, Some(path))
            .map_err(mlua::Error::external)?;
        engine.lua_component_field(actor, stable_id, path)
            .map_err(mlua::Error::external)
    })
}

pub(super) fn set_component_root(target: LuaEntityTarget, component: &str, value: DynamicValue) -> mlua::Result<()> {
    with_live_actor(target, |engine, actor| {
        let stable_id = engine.ensure_lua_component_access(component, Some(&FieldPath::root()))
            .map_err(mlua::Error::external)?;
        engine.set_lua_component_value(actor, stable_id, value)
            .map_err(mlua::Error::external)
    })
}

pub(super) fn set_component_value(
    target: LuaEntityTarget,
    component: &str,
    path: &FieldPath,
    value: DynamicValue,
) -> mlua::Result<()> {
    with_live_actor(target, |engine, actor| {
        let stable_id = engine.ensure_lua_component_access(component, Some(path))
            .map_err(mlua::Error::external)?;
        engine.set_lua_component_field(actor, stable_id, path, value)
            .map_err(mlua::Error::external)
    })
}

pub(super) fn append_text_path(base: &FieldPath, suffix: &str) -> mlua::Result<FieldPath> {
    let mut segments = base.segments().to_vec();
    if !suffix.is_empty() {
        segments.extend(FieldPath::parse(suffix).map_err(mlua::Error::external)?.segments().iter().cloned());
    }
    Ok(FieldPath::new(segments))
}

pub(super) fn is_dynamic_container(value: &DynamicValue) -> bool {
    matches!(value, DynamicValue::Object(_) | DynamicValue::Array(_))
}

pub(super) fn lua_key_is_value(key: &Value) -> bool {
    match key {
        Value::String(value) => value.to_string_lossy() == "value",
        _ => false,
    }
}

pub(super) fn child_path_for_lua_key(base: &FieldPath, current: &DynamicValue, key: Value) -> mlua::Result<FieldPath> {
    let mut path = FieldPath::new(base.segments().to_vec());
    match key {
        Value::Integer(index) => {
            if index < 1 { return Err(mlua::Error::external("Lua component array indexes start at 1")); }
            path.push_index((index - 1) as usize);
        }
        Value::Number(index) if index.fract() == 0.0 && index >= 1.0 => {
            path.push_index(index as usize - 1);
        }
        Value::String(name) => {
            let name = name.to_string_lossy();
            if let DynamicValue::Array(values) = current {
                let vector_index = match name.as_str() {
                    "x" | "r" if !values.is_empty() => Some(0),
                    "y" | "g" if values.len() > 1 => Some(1),
                    "z" | "b" if values.len() > 2 => Some(2),
                    "w" | "a" if values.len() > 3 => Some(3),
                    _ => None,
                };
                if let Some(index) = vector_index {
                    path.push_index(index);
                } else {
                    return Err(mlua::Error::external(format!("array has no field `{name}`")));
                }
            } else {
                path.push_field(name.as_str());
            }
        }
        other => return Err(mlua::Error::external(format!("invalid component key: {other:?}"))),
    }
    Ok(path)
}

pub(super) fn dynamic_to_lua_table(lua: &Lua, value: &DynamicValue) -> mlua::Result<Value> {
    match value {
        DynamicValue::Null => Ok(Value::Nil),
        DynamicValue::Bool(value) => Ok(Value::Boolean(*value)),
        DynamicValue::I64(value) => Ok(Value::Integer(*value)),
        DynamicValue::U64(value) if *value <= i64::MAX as u64 => Ok(Value::Integer(*value as i64)),
        DynamicValue::U64(value) => Ok(Value::Number(*value as f64)),
        DynamicValue::F64(value) => Ok(Value::Number(*value)),
        DynamicValue::String(value) => Ok(Value::String(lua.create_string(value)?)),
        DynamicValue::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.set(index + 1, dynamic_to_lua_table(lua, value)?)?;
            }
            Ok(Value::Table(table))
        }
        DynamicValue::Object(values) => {
            let table = lua.create_table()?;
            for (name, value) in values {
                table.set(name.as_str(), dynamic_to_lua_table(lua, value)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}

pub(super) fn lua_to_dynamic(value: Value) -> mlua::Result<DynamicValue> {
    match value {
        Value::Nil => Ok(DynamicValue::Null),
        Value::Boolean(value) => Ok(DynamicValue::Bool(value)),
        Value::Integer(value) => Ok(DynamicValue::I64(value)),
        Value::Number(value) => Ok(DynamicValue::F64(value)),
        Value::String(value) => Ok(DynamicValue::String(value.to_string_lossy().to_string())),
        Value::Table(table) => table_to_dynamic(table),
        other => Err(mlua::Error::external(format!("unsupported reflected component value: {other:?}"))),
    }
}

pub(super) fn table_to_dynamic(table: Table) -> mlua::Result<DynamicValue> {
    let mut entries = Vec::new();
    let mut integer_only = true;
    let mut max_index = 0usize;
    for pair in table.pairs::<Value, Value>() {
        let (key, value) = pair?;
        match &key {
            Value::Integer(index) if *index >= 1 => max_index = max_index.max(*index as usize),
            _ => integer_only = false,
        }
        entries.push((key, value));
    }
    if integer_only && !entries.is_empty() && max_index == entries.len() {
        let mut values = vec![DynamicValue::Null; max_index];
        for (key, value) in entries {
            let Value::Integer(index) = key else { unreachable!() };
            values[index as usize - 1] = lua_to_dynamic(value)?;
        }
        return Ok(DynamicValue::Array(values));
    }
    let mut values = BTreeMap::new();
    for (key, value) in entries {
        let name = match key {
            Value::String(value) => value.to_string_lossy().to_string(),
            Value::Integer(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            other => return Err(mlua::Error::external(format!("unsupported reflected object key: {other:?}"))),
        };
        values.insert(name, lua_to_dynamic(value)?);
    }
    Ok(DynamicValue::Object(values))
}
