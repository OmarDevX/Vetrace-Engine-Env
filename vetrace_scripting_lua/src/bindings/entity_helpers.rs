use super::*;

pub(super) fn queue_audio(
    lua: &Lua,
    path: String,
    position: Option<glam::Vec3>,
    volume: f32,
    looping: bool,
) -> mlua::Result<AnyUserData> {
    let request = allocate_spawn_request()?;
    let volume = volume * crate::runtime_api::master_volume()?;
    queue_command(LuaCommand::PlayAudio {
        request,
        path,
        position,
        volume: volume.clamp(0.0, 4.0),
        looping,
    })?;
    lua.create_userdata(EntityProxy::pending(request))
}

pub(super) fn spawn_pending(lua: &Lua, name: String) -> mlua::Result<AnyUserData> {
    let request = allocate_spawn_request()?;
    queue_command(LuaCommand::Spawn { request, name })?;
    lua.create_userdata(EntityProxy::pending(request))
}

pub(super) fn find_entity_by_name(lua: &Lua, name: &str) -> mlua::Result<Option<AnyUserData>> {
    with_context(|engine, _, _, _, _, _| {
        match engine.find_actor_by_name(name) {
            Some(actor) => Ok(Some(lua.create_userdata(EntityProxy::live(actor.entity()))?)),
            None => Ok(None),
        }
    })
}

pub(super) fn find_entity_by_tag(lua: &Lua, tag: &str) -> mlua::Result<Option<AnyUserData>> {
    with_context(|engine, _, _, _, _, _| {
        for entity in engine.raw_world().entities() {
            if engine.actor(entity).is_some_and(|actor| actor.has_tag(engine, tag)) {
                return Ok(Some(lua.create_userdata(EntityProxy::live(entity))?));
            }
        }
        Ok(None)
    })
}

pub(super) fn find_all_entities_by_tag(lua: &Lua, tag: &str) -> mlua::Result<Table> {
    with_context(|engine, _, _, _, _, _| {
        let table = lua.create_table()?;
        let mut index = 1;
        for entity in engine.raw_world().entities() {
            if engine.actor(entity).is_some_and(|actor| actor.has_tag(engine, tag)) {
                table.set(index, lua.create_userdata(EntityProxy::live(entity))?)?;
                index += 1;
            }
        }
        Ok(table)
    })
}

pub(super) fn set_entity_name(target: LuaEntityTarget, name: String) -> mlua::Result<()> {
    match target {
        LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
            if let Some(actor) = engine.actor(entity) {
                actor.set_name(engine, name).map_err(mlua::Error::external)?;
            }
            Ok(())
        }),
        LuaEntityTarget::Pending(request) => queue_command(LuaCommand::SetPendingName { request, name }),
    }
}

pub(super) fn add_entity_tag(target: LuaEntityTarget, tag: String) -> mlua::Result<()> {
    match target {
        LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
            if let Some(actor) = engine.actor(entity) {
                actor.add_tag(engine, tag).map_err(mlua::Error::external)?;
            }
            Ok(())
        }),
        LuaEntityTarget::Pending(request) => queue_command(LuaCommand::AddPendingTag { request, tag }),
    }
}


pub(super) fn with_live_actor<R>(
    target: LuaEntityTarget,
    operation: impl FnOnce(&mut vetrace_core::Engine, vetrace_core::Actor) -> mlua::Result<R>,
) -> mlua::Result<R> {
    with_context(|engine, _, _, _, _, _| {
        let entity = resolve_entity_target(engine, target).ok_or_else(|| {
            mlua::Error::external("entity has not spawned yet or is no longer alive")
        })?;
        let actor = engine.actor(entity).ok_or_else(|| {
            mlua::Error::external(format!("entity {} is no longer alive", entity.0))
        })?;
        operation(engine, actor)
    })
}

pub(super) fn resolve_entity_now(target: LuaEntityTarget) -> mlua::Result<Option<Entity>> {
    with_context(|engine, _, _, _, _, _| Ok(resolve_entity_target(engine, target)))
}

pub(super) fn has_component(target: LuaEntityTarget, component: &str) -> mlua::Result<bool> {
    with_context(|engine, _, _, _, _, _| {
        let Some(entity) = resolve_entity_target(engine, target) else { return Ok(false); };
        let Some(actor) = engine.actor(entity) else { return Ok(false); };
        let stable_id = match engine.ensure_lua_component_access(component, None) {
            Ok(id) => id,
            Err(vetrace_core::ReflectionError::ComponentNotRegistered(_)) => return Ok(false),
            Err(error) => return Err(mlua::Error::external(error)),
        };
        Ok(engine.registered_component_value(actor, stable_id).is_ok())
    })
}

pub(super) fn component_proxy(
    lua: &Lua,
    target: LuaEntityTarget,
    component: &str,
    require_present: bool,
) -> mlua::Result<Value> {
    let stable_id = with_context(|engine, _, _, _, _, _| {
        match engine.ensure_lua_component_access(component, None) {
            Ok(id) => Ok(Some(id.to_owned())),
            Err(vetrace_core::ReflectionError::ComponentNotRegistered(_)) => Ok(None),
            Err(error) => Err(mlua::Error::external(error)),
        }
    })?;
    let Some(stable_id) = stable_id else { return Ok(Value::Nil); };
    if require_present && !has_component(target, &stable_id)? { return Ok(Value::Nil); }
    Ok(Value::UserData(lua.create_userdata(DynamicComponentProxy::root(target, stable_id))?))
}

pub(super) fn component_ids(lua: &Lua, target: LuaEntityTarget) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    with_context(|engine, _, _, _, _, _| {
        if let Some(entity) = resolve_entity_target(engine, target) {
            if let Some(actor) = engine.actor(entity) {
                for (index, id) in engine.lua_components(actor).into_iter().enumerate() {
                    table.set(index + 1, id)?;
                }
            }
        }
        Ok(())
    })?;
    Ok(table)
}

pub(super) fn queue_add_component(target: LuaEntityTarget, component: String, value: Option<Value>) -> mlua::Result<()> {
    let component = with_context(|engine, _, _, _, _, _| {
        engine.ensure_lua_component_access(&component, None)
            .map(str::to_owned)
            .map_err(mlua::Error::external)
    })?;
    let value = value.map(lua_to_dynamic).transpose()?;
    queue_command(LuaCommand::AddComponent { target, component, value })
}

pub(super) fn queue_remove_component(target: LuaEntityTarget, component: String) -> mlua::Result<()> {
    let component = with_context(|engine, _, _, _, _, _| {
        engine.ensure_lua_component_access(&component, None)
            .map(str::to_owned)
            .map_err(mlua::Error::external)
    })?;
    queue_command(LuaCommand::RemoveComponent { target, component })
}
