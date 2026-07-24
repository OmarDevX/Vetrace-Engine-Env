use super::*;

pub(super) fn debug_value_summary(value: &LuaDebugValue) -> String {
    match value {
        LuaDebugValue::Nil => "nil".to_owned(),
        LuaDebugValue::Boolean(value) => value.to_string(),
        LuaDebugValue::Integer(value) => value.to_string(),
        LuaDebugValue::Number(value) => value.to_string(),
        LuaDebugValue::String(value) => format!("\"{value}\""),
        LuaDebugValue::Table(values) => {
            let preview = values.iter().take(4)
                .map(|value| format!("{}={}", value.name, debug_value_summary(&value.value)))
                .collect::<Vec<_>>()
                .join(", ");
            if values.len() > 4 { format!("{{{preview}, …}}") } else { format!("{{{preview}}}") }
        }
        LuaDebugValue::Function => "<function>".to_owned(),
        LuaDebugValue::UserData(name) => format!("<{name}>") ,
        LuaDebugValue::Thread => "<thread>".to_owned(),
        LuaDebugValue::Error(message) => format!("<error: {message}>") ,
        LuaDebugValue::Other(value) => format!("<{value}>") ,
    }
}
