use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let assets = lua.create_table()?;
    assets.set("exists", lua.create_function(|_, path: String| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine
                .get_resource::<crate::LuaProjectContext>()
                .is_some_and(|context| context.exists(&path)))
        })
    })?)?;
    assets.set("read_text", lua.create_function(|_, path: String| {
        with_context(|engine, _, _, _, _, _| {
            let context = engine
                .get_resource::<crate::LuaProjectContext>()
                .ok_or_else(|| mlua::Error::external("Lua project context is unavailable"))?;
            let resolved = context.resolve_existing(&path).map_err(mlua::Error::external)?;
            std::fs::read_to_string(&resolved).map_err(mlua::Error::external)
        })
    })?)?;
    env.set("Assets", assets)?;
    Ok(())
}
