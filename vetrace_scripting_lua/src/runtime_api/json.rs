use super::*;

pub(super) fn install_json_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let json = lua.create_table()?;
    json.set("encode", lua.create_function(|_, value: Value| {
        let value = lua_to_json(value, 0)?;
        serde_json::to_string(&value).map_err(mlua::Error::external)
    })?)?;
    json.set("decode", lua.create_function(|lua, text: String| {
        let value: JsonValue = serde_json::from_str(&text).map_err(mlua::Error::external)?;
        json_to_lua(lua, &value, 0)
    })?)?;
    env.set("Json", json)
}

pub(super) fn lua_to_json(value: Value, depth: usize) -> mlua::Result<JsonValue> {
    if depth > MAX_JSON_DEPTH { return Err(mlua::Error::external("JSON value exceeds maximum nesting depth")); }
    Ok(match value {
        Value::Nil => JsonValue::Null,
        Value::Boolean(value) => JsonValue::Bool(value),
        Value::Integer(value) => JsonValue::Number(JsonNumber::from(value)),
        Value::Number(value) => JsonNumber::from_f64(value).map(JsonValue::Number).ok_or_else(|| mlua::Error::external("JSON cannot encode NaN or infinity"))?,
        Value::String(value) => JsonValue::String(value.to_str()?.to_owned()),
        Value::Table(table) => {
            let mut integer_keys = Vec::<(usize, JsonValue)>::new();
            let mut object = JsonMap::new();
            let mut array_candidate = true;
            for pair in table.pairs::<Value, Value>() {
                let (key, value) = pair?;
                let value = lua_to_json(value, depth + 1)?;
                match key {
                    Value::Integer(index) if index > 0 => integer_keys.push((index as usize, value)),
                    Value::String(key) => {
                        array_candidate = false;
                        object.insert(key.to_str()?.to_owned(), value);
                    }
                    other => return Err(mlua::Error::external(format!("JSON object key must be a string or positive integer, got {}", other.type_name()))),
                }
            }
            integer_keys.sort_by_key(|(index, _)| *index);
            if array_candidate && integer_keys.iter().enumerate().all(|(offset, (index, _))| *index == offset + 1) {
                JsonValue::Array(integer_keys.into_iter().map(|(_, value)| value).collect())
            } else {
                for (index, value) in integer_keys { object.insert(index.to_string(), value); }
                JsonValue::Object(object)
            }
        }
        other => return Err(mlua::Error::external(format!("JSON cannot encode Lua {}", other.type_name()))),
    })
}

pub(super) fn json_to_lua(lua: &Lua, value: &JsonValue, depth: usize) -> mlua::Result<Value> {
    if depth > MAX_JSON_DEPTH { return Err(mlua::Error::external("JSON value exceeds maximum nesting depth")); }
    Ok(match value {
        JsonValue::Null => Value::Nil,
        JsonValue::Bool(value) => Value::Boolean(*value),
        JsonValue::Number(value) => {
            if let Some(integer) = value.as_i64() { Value::Integer(integer) }
            else { Value::Number(value.as_f64().unwrap_or_default()) }
        }
        JsonValue::String(value) => Value::String(lua.create_string(value)?),
        JsonValue::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() { table.set(index + 1, json_to_lua(lua, value, depth + 1)?)?; }
            Value::Table(table)
        }
        JsonValue::Object(values) => {
            let table = lua.create_table()?;
            for (key, value) in values { table.set(key.as_str(), json_to_lua(lua, value, depth + 1)?)?; }
            Value::Table(table)
        }
    })
}
