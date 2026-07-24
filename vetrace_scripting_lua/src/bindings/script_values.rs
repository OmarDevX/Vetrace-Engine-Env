use super::*;

pub(super) fn lua_to_script_value(value: Value) -> mlua::Result<crate::ScriptValue> {
    match value {
        Value::Nil => Ok(crate::ScriptValue::Null),
        Value::Boolean(value) => Ok(crate::ScriptValue::Bool(value)),
        Value::Integer(value) => Ok(crate::ScriptValue::Integer(value)),
        Value::Number(value) => Ok(crate::ScriptValue::Number(value)),
        Value::String(value) => Ok(crate::ScriptValue::String(value.to_string_lossy().to_string())),
        Value::Table(table) => {
            let values = table.sequence_values::<f32>().collect::<mlua::Result<Vec<_>>>()?;
            match values.as_slice() {
                [x, y] => Ok(crate::ScriptValue::Vec2([*x, *y])),
                [x, y, z] => Ok(crate::ScriptValue::Vec3([*x, *y, *z])),
                [x, y, z, w] => Ok(crate::ScriptValue::Vec4([*x, *y, *z, *w])),
                _ => Err(mlua::Error::external(
                    "event payload tables must be numeric vec2, vec3, or vec4 arrays",
                )),
            }
        }
        other => Err(mlua::Error::external(format!(
            "unsupported event payload: {other:?}"
        ))),
    }
}

pub(super) fn lua_number(value: Value) -> Option<f32> {
    match value {
        Value::Integer(value) => Some(value as f32),
        Value::Number(value) => Some(value as f32),
        _ => None,
    }
}

pub(super) fn print_lua_value(value: Value) { println!("{}", display_lua_value(&value)); }

pub(super) fn display_lua_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.to_string_lossy().to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Boolean(value) => value.to_string(),
        Value::Nil => "nil".to_owned(),
        other => format!("{other:?}"),
    }
}
