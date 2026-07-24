use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let events = lua.create_table()?;
    events.set("emit", lua.create_function(|_, (entity, name, payload): (AnyUserData, String, Option<Value>)| {
        let proxy = entity.borrow::<EntityProxy>()?;
        let Some(entity) = resolve_entity_now(proxy.target)? else {
            return Err(mlua::Error::external("events cannot target an entity that has not spawned yet"));
        };
        queue_command(LuaCommand::EmitEvent {
            target: Some(entity),
            name,
            payload: lua_to_script_value(payload.unwrap_or(Value::Nil))?,
        })
    })?)?;
    events.set("broadcast", lua.create_function(|_, (name, payload): (String, Option<Value>)| {
        queue_command(LuaCommand::EmitEvent {
            target: None,
            name,
            payload: lua_to_script_value(payload.unwrap_or(Value::Nil))?,
        })
    })?)?;
    env.set("Events", events)?;
    Ok(())
}
