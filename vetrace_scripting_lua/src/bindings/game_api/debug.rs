use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let debug = lua.create_table()?;
    debug.set("log", lua.create_function(|_, value: Value| {
        print_lua_value(value);
        Ok(())
    })?)?;
    debug.set("warn", lua.create_function(|_, value: Value| {
        eprintln!("Lua warning: {}", display_lua_value(&value));
        Ok(())
    })?)?;
    debug.set("error", lua.create_function(|_, value: Value| {
        eprintln!("Lua error: {}", display_lua_value(&value));
        Ok(())
    })?)?;
    env.set("Debug", debug)?;
    Ok(())
}
