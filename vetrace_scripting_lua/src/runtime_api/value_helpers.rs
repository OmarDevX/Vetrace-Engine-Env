use super::*;

pub(super) fn expect_string(value: Value, name: &str) -> mlua::Result<String> {
    match value { Value::String(value) => Ok(value.to_str()?.to_owned()), other => Err(mlua::Error::external(format!("{name} expects a string, got {}", other.type_name()))) }
}

pub(super) fn expect_bool(value: Value, name: &str) -> mlua::Result<bool> {
    match value { Value::Boolean(value) => Ok(value), other => Err(mlua::Error::external(format!("{name} expects a boolean, got {}", other.type_name()))) }
}

pub(super) fn expect_number(value: Value, name: &str) -> mlua::Result<f32> {
    match value {
        Value::Integer(value) => Ok(value as f32),
        Value::Number(value) if value.is_finite() => Ok(value as f32),
        other => Err(mlua::Error::external(format!("{name} expects a finite number, got {}", other.type_name()))),
    }
}
