use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let entity = lua.create_table()?;
    entity.set("current", lua.create_function(|lua, ()| {
        with_context(|_, _, _, current, _, _| {
            match current {
                Some(entity) => Ok(Some(lua.create_userdata(EntityProxy::live(entity))?)),
                None => Ok(None),
            }
        })
    })?)?;
    env.set("Entity", entity)?;
    Ok(())
}
