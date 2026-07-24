use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let scene = lua.create_table()?;
    scene.set("spawn", lua.create_function(|lua, name: Option<String>| {
        spawn_pending(lua, name.unwrap_or_else(|| "Actor".to_owned()))
    })?)?;
    scene.set("instantiate", lua.create_function(|lua, path: String| {
        let request = allocate_spawn_request()?;
        queue_command(LuaCommand::InstantiateScene { request, path })?;
        lua.create_userdata(EntityProxy::pending(request))
    })?)?;
    scene.set("destroy", lua.create_function(|_, entity: AnyUserData| {
        let proxy = entity.borrow::<EntityProxy>()?;
        if let Some(entity) = resolve_entity_now(proxy.target)? {
            queue_command(LuaCommand::Destroy(entity))?;
        }
        Ok(())
    })?)?;
    scene.set("find_by_name", lua.create_function(|lua, name: String| find_entity_by_name(lua, &name))?)?;
    scene.set("find_by_tag", lua.create_function(|lua, tag: String| find_entity_by_tag(lua, &tag))?)?;
    scene.set("find_all_by_tag", lua.create_function(|lua, tag: String| find_all_entities_by_tag(lua, &tag))?)?;
    scene.set("entity_count", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| Ok(engine.raw_world().entities().count() as u64))
    })?)?;
    scene.set("clear", lua.create_function(|_, ()| queue_command(LuaCommand::ClearScene))?)?;
    env.set("Scene", scene)?;
    Ok(())
}
