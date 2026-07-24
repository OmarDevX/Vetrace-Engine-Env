use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let time = lua.create_table()?;
    time.set("delta", lua.create_function(|_, ()| {
        with_context(|_, _, _, _, dt, _| Ok(dt))
    })?)?;
    time.set("is_fixed_update", lua.create_function(|_, ()| {
        with_context(|_, _, _, _, _, fixed| Ok(fixed))
    })?)?;
    env.set("Time", time)?;
    Ok(())
}
