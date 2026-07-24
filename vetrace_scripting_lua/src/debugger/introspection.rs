use super::*;

pub(super) fn normalize_path(path: &str) -> String {
    path.trim_start_matches('@').replace('\\', "/")
}

pub(super) fn stack_depth(lua: &Lua) -> usize {
    let mut depth = 0;
    while lua.inspect_stack(depth, |_| ()).is_some() {
        depth += 1;
        if depth >= 128 { break; }
    }
    depth
}

pub(super) fn collect_stack(lua: &Lua) -> Vec<LuaStackFrame> {
    let mut frames = Vec::new();
    for level in 0..128 {
        let Some(frame) = lua.inspect_stack(level, |debug| {
            let names = debug.names();
            let source = debug.source();
            LuaStackFrame {
                name: names
                    .name
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| source.what.to_owned()),
                source: source
                    .source
                    .as_ref()
                    .or(source.short_src.as_ref())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "<unknown>".to_owned())
                    .trim_start_matches('@')
                    .replace('\\', "/"),
                line: debug.current_line(),
                defined_line: source.line_defined,
            }
        }) else {
            break;
        };
        frames.push(frame);
    }
    frames
}

pub(super) fn collect_local_table(lua: &Lua) -> Option<Table> {
    // mlua intentionally keeps raw stack access out of its normal safe API.
    // `exec_raw` locks the state and restores the stack after conversion, so
    // the unsafe block is limited to copying locals into a new Lua table. The
    // hook itself may occupy stack level 0 on some mlua/Lua combinations, so
    // scan outward and use the first activation that has real named locals.
    // No raw pointer or stack index escapes this function.
    unsafe {
        lua.exec_raw::<Table>((), |state| {
            mlua::ffi::lua_createtable(state, 0, 16);
            let table_index = mlua::ffi::lua_gettop(state);

            for level in 0..128 {
                let mut activation = std::mem::zeroed::<mlua::ffi::lua_Debug>();
                if mlua::ffi::lua_getstack(state, level, &mut activation) == 0 {
                    break;
                }

                let mut captured = 0usize;
                for index in 1..=512 {
                    let name = mlua::ffi::lua_getlocal(state, &activation, index);
                    if name.is_null() {
                        break;
                    }
                    let name = CStr::from_ptr(name).to_bytes();
                    if name.is_empty() || name.starts_with(b"(") {
                        mlua::ffi::lua_pop(state, 1);
                        continue;
                    }
                    mlua::ffi::lua_pushlstring(state, name.as_ptr().cast(), name.len());
                    mlua::ffi::lua_insert(state, -2);
                    mlua::ffi::lua_rawset(state, table_index);
                    captured += 1;
                }

                if captured > 0 {
                    break;
                }
            }
        }).ok()
    }
}

pub(super) fn collect_debug_variables(lua: &Lua, locals: Option<&Table>) -> Vec<LuaDebugVariable> {
    let mut values = locals.map(|table| table_variables(table, 0)).unwrap_or_default();
    if let Ok(Value::Table(table)) = lua.named_registry_value::<Value>(DEBUG_SELF_REGISTRY_KEY) {
        let self_value = LuaDebugVariable {
            name: "self".to_owned(),
            value: LuaDebugValue::Table(table_variables(&table, 0)),
        };
        values.retain(|variable| variable.name != "self");
        values.push(self_value);
    }
    values.sort_by(|left, right| left.name.cmp(&right.name));
    values
}

pub(super) fn evaluate_watches(
    lua: &Lua,
    expressions: &[String],
    locals: Option<&Table>,
) -> Vec<LuaDebugVariable> {
    expressions
        .iter()
        .map(|expression| LuaDebugVariable {
            name: expression.clone(),
            value: evaluate_watch(lua, expression, locals)
                .unwrap_or_else(|message| LuaDebugValue::Error(message)),
        })
        .collect()
}

fn evaluate_watch(
    lua: &Lua,
    expression: &str,
    locals: Option<&Table>,
) -> Result<LuaDebugValue, String> {
    let mut parts = expression.split('.').filter(|part| !part.is_empty());
    let Some(root) = parts.next() else { return Err("empty watch expression".to_owned()); };
    let mut value = if root == "self" {
        lua.named_registry_value::<Value>(DEBUG_SELF_REGISTRY_KEY)
            .map_err(|error| error.to_string())?
    } else if let Some(value) = local_value(locals, root)? {
        value
    } else {
        lua.globals().get::<Value>(root).map_err(|error| error.to_string())?
    };
    for part in parts {
        let Value::Table(table) = value else {
            return Err(format!("'{part}' cannot be read from {}", value.type_name()));
        };
        value = table.get::<Value>(part).map_err(|error| error.to_string())?;
    }
    Ok(value_to_debug(value, 0))
}

pub(super) fn local_value(locals: Option<&Table>, name: &str) -> Result<Option<Value>, String> {
    let Some(locals) = locals else { return Ok(None); };
    for pair in locals.clone().pairs::<Value, Value>() {
        let (key, value) = pair.map_err(|error| error.to_string())?;
        if matches!(key, Value::String(ref key) if key.to_string_lossy() == name) {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

pub(super) fn table_variables(table: &Table, depth: usize) -> Vec<LuaDebugVariable> {
    if depth >= 3 { return Vec::new(); }
    let mut values = Vec::new();
    for pair in table.clone().pairs::<Value, Value>() {
        let Ok((key, value)) = pair else { continue; };
        let name = match key {
            Value::String(value) => value.to_string_lossy().to_string(),
            Value::Integer(value) => value.to_string(),
            Value::Number(value) => value.to_string(),
            other => format!("<{:?}>", other.type_name()),
        };
        values.push(LuaDebugVariable {
            name,
            value: value_to_debug(value, depth + 1),
        });
        if values.len() >= 128 { break; }
    }
    values.sort_by(|left, right| left.name.cmp(&right.name));
    values
}

pub(super) fn value_to_debug(value: Value, depth: usize) -> LuaDebugValue {
    match value {
        Value::Nil => LuaDebugValue::Nil,
        Value::Boolean(value) => LuaDebugValue::Boolean(value),
        Value::Integer(value) => LuaDebugValue::Integer(value),
        Value::Number(value) => LuaDebugValue::Number(value),
        Value::String(value) => LuaDebugValue::String(value.to_string_lossy().to_string()),
        Value::Table(table) if depth < 3 => LuaDebugValue::Table(table_variables(&table, depth)),
        Value::Table(_) => LuaDebugValue::Other("table …".to_owned()),
        Value::Function(_) => LuaDebugValue::Function,
        // `AnyUserData::type_name` was not public in mlua 0.11.2. Keep
        // the protocol compatible with that exact dependency version instead
        // of relying on a newer mlua API.
        Value::UserData(_) => LuaDebugValue::UserData("userdata".to_owned()),
        Value::Thread(_) => LuaDebugValue::Thread,
        Value::Error(error) => LuaDebugValue::Error(error.to_string()),
        other => LuaDebugValue::Other(other.type_name().to_owned()),
    }
}

pub(super) fn parse_error_line(message: &str) -> Option<usize> {
    message
        .split(':')
        .find_map(|piece| piece.trim().parse::<usize>().ok())
}
