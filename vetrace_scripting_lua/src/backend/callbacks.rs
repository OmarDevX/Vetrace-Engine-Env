use super::*;

pub(super) fn invoke_entity_callback(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    entity: Entity,
    callback: EntityCallback,
    dt: f32,
    fixed_update: bool,
) -> Result<Vec<LuaCommand>, String> {
    let Some(instance) = state.instances.get(&entity) else { return Ok(Vec::new()); };
    let Some(script) = state.scripts.get(&instance.script) else { return Ok(Vec::new()); };
    let key = match callback {
        EntityCallback::Ready => script.callbacks.ready.as_ref(),
        EntityCallback::Update => script.callbacks.update.as_ref(),
        EntityCallback::FixedUpdate => script.callbacks.fixed_update.as_ref(),
        EntityCallback::Destroy => script.callbacks.destroy.as_ref(),
    };
    let Some(key) = key else { return Ok(Vec::new()); };

    let function = state.lua.registry_value::<Function>(key).map_err(|error| error.to_string())?;
    let instance_table = state
        .lua
        .registry_value::<Table>(&instance.table)
        .map_err(|error| error.to_string())?;
    let debug_path = instance.script.clone();
    let debugger = begin_debug_callback(
        state,
        &debug_path,
        callback.name(),
        Some(entity),
        &instance_table,
    );
    let mut commands = Vec::new();
    let result = scope_context(
        engine,
        &mut commands,
        Some(entity),
        dt,
        fixed_update,
        &mut state.next_spawn_request,
        || match script.style {
            LuaScriptStyle::Gameplay => match callback {
                EntityCallback::Ready | EntityCallback::Destroy => function.call::<()>(instance_table),
                EntityCallback::Update | EntityCallback::FixedUpdate => function.call::<()>((instance_table, dt)),
            },
            LuaScriptStyle::Legacy => match callback {
                EntityCallback::Ready | EntityCallback::Destroy => {
                    function.call::<()>((EngineHandle, EntityProxy::live(entity)))
                }
                EntityCallback::Update | EntityCallback::FixedUpdate => function.call::<()>(
                    (EngineHandle, EntityProxy::live(entity), InputProxy::new(), dt),
                ),
            },
        },
    );
    finish_debug_callback(state, debugger, &result);
    result.map_err(|error| error.to_string())?;
    Ok(commands)
}

pub(super) fn invoke_autoload_callback(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    name: &str,
    callback: EntityCallback,
    dt: f32,
    fixed_update: bool,
) -> Result<Vec<LuaCommand>, String> {
    let Some(script) = state.scripts.get(name) else { return Ok(Vec::new()); };
    let key = match callback {
        EntityCallback::Ready => script.callbacks.ready.as_ref(),
        EntityCallback::Update => script.callbacks.update.as_ref(),
        EntityCallback::FixedUpdate => script.callbacks.fixed_update.as_ref(),
        EntityCallback::Destroy => script.callbacks.destroy.as_ref(),
    };
    let Some(key) = key else { return Ok(Vec::new()); };
    let Some(instance) = state.autoload_instances.get(name) else { return Ok(Vec::new()); };

    let function = state.lua.registry_value::<Function>(key).map_err(|error| error.to_string())?;
    let instance_table = state
        .lua
        .registry_value::<Table>(&instance.table)
        .map_err(|error| error.to_string())?;
    let debugger = begin_debug_callback(
        state,
        name,
        callback.name(),
        None,
        &instance_table,
    );
    let mut commands = Vec::new();
    let result = scope_context(
        engine,
        &mut commands,
        None,
        dt,
        fixed_update,
        &mut state.next_spawn_request,
        || match script.style {
            LuaScriptStyle::Gameplay => match callback {
                EntityCallback::Ready | EntityCallback::Destroy => function.call::<()>(instance_table),
                EntityCallback::Update | EntityCallback::FixedUpdate => function.call::<()>((instance_table, dt)),
            },
            LuaScriptStyle::Legacy => match callback {
                EntityCallback::Ready | EntityCallback::Destroy => function.call::<()>(EngineHandle),
                EntityCallback::Update | EntityCallback::FixedUpdate => {
                    function.call::<()>((EngineHandle, InputProxy::new(), dt))
                }
            },
        },
    );
    finish_debug_callback(state, debugger, &result);
    result.map_err(|error| error.to_string())?;
    Ok(commands)
}

pub(super) fn invoke_autoload_event(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    name: &str,
    event: &str,
    payload: &ScriptValue,
) -> Result<Vec<LuaCommand>, String> {
    let Some(instance) = state.autoload_instances.get(name) else { return Ok(Vec::new()); };
    if instance.status != LuaScriptInstanceStatus::Running { return Ok(Vec::new()); }
    let Some(script) = state.scripts.get(name) else { return Ok(Vec::new()); };
    let Some(key) = script.callbacks.event.as_ref() else { return Ok(Vec::new()); };
    let function = state.lua.registry_value::<Function>(key).map_err(|error| error.to_string())?;
    let table = state.lua.registry_value::<Table>(&instance.table).map_err(|error| error.to_string())?;
    let payload = script_value_to_lua(&state.lua, payload).map_err(|error| error.to_string())?;
    let debugger = begin_debug_callback(state, name, "on_event", None, &table);
    let mut commands = Vec::new();
    let result = scope_context(
        engine,
        &mut commands,
        None,
        0.0,
        false,
        &mut state.next_spawn_request,
        || function.call::<()>((table, event.to_owned(), payload)),
    );
    finish_debug_callback(state, debugger, &result);
    result.map_err(|error| error.to_string())?;
    Ok(commands)
}

pub(super) fn invoke_entity_event(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    entity: Entity,
    event: &str,
    payload: &ScriptValue,
) -> Result<Vec<LuaCommand>, String> {
    let Some(instance) = state.instances.get(&entity) else { return Ok(Vec::new()); };
    if instance.status != LuaScriptInstanceStatus::Running { return Ok(Vec::new()); }
    let Some(script) = state.scripts.get(&instance.script) else { return Ok(Vec::new()); };
    let Some(key) = script.callbacks.event.as_ref() else { return Ok(Vec::new()); };
    let function = state.lua.registry_value::<Function>(key).map_err(|error| error.to_string())?;
    let table = state
        .lua
        .registry_value::<Table>(&instance.table)
        .map_err(|error| error.to_string())?;
    let payload = script_value_to_lua(&state.lua, payload).map_err(|error| error.to_string())?;
    let debug_path = instance.script.clone();
    let debugger = begin_debug_callback(
        state,
        &debug_path,
        "on_event",
        Some(entity),
        &table,
    );
    let mut commands = Vec::new();
    let result = scope_context(
        engine,
        &mut commands,
        Some(entity),
        0.0,
        false,
        &mut state.next_spawn_request,
        || function.call::<()>((table, event.to_owned(), payload)),
    );
    finish_debug_callback(state, debugger, &result);
    result.map_err(|error| error.to_string())?;
    Ok(commands)
}

pub(super) fn dispatch_collision(engine: &mut Engine, entity: Entity, other: Entity, entering: bool) {
    ensure_resources(engine);
    with_state(engine, |engine, state| {
        let result = invoke_entity_collision(engine, state, entity, other, entering);
        match result {
            Ok(commands) => flush_lua_commands(engine, state, commands),
            Err(message) => {
                let script = state
                    .entity_scripts
                    .get(&entity)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_owned());
                mark_entity_failed(state, entity, &message);
                record_error(
                    engine,
                    LuaDiagnosticTarget::Entity { entity, script },
                    if entering { "on_collision_enter" } else { "on_collision_exit" },
                    message,
                );
            }
        }
    });
}

pub(super) fn invoke_entity_collision(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    entity: Entity,
    other: Entity,
    entering: bool,
) -> Result<Vec<LuaCommand>, String> {
    let Some(instance) = state.instances.get(&entity) else { return Ok(Vec::new()); };
    if instance.status != LuaScriptInstanceStatus::Running { return Ok(Vec::new()); }
    let Some(script) = state.scripts.get(&instance.script) else { return Ok(Vec::new()); };
    let key = if entering {
        script.callbacks.collision_enter.as_ref().or(script.callbacks.legacy_collision.as_ref())
    } else {
        script.callbacks.collision_exit.as_ref()
    };
    let Some(key) = key else { return Ok(Vec::new()); };
    let function = state.lua.registry_value::<Function>(key).map_err(|error| error.to_string())?;
    let table = state
        .lua
        .registry_value::<Table>(&instance.table)
        .map_err(|error| error.to_string())?;
    let debug_path = instance.script.clone();
    let callback_name = if entering { "on_collision_enter" } else { "on_collision_exit" };
    let debugger = begin_debug_callback(
        state,
        &debug_path,
        callback_name,
        Some(entity),
        &table,
    );
    let mut commands = Vec::new();
    let result = scope_context(
        engine,
        &mut commands,
        Some(entity),
        0.0,
        false,
        &mut state.next_spawn_request,
        || match script.style {
            LuaScriptStyle::Gameplay => function
                .call::<()>((table, EntityProxy::live(other))),
            LuaScriptStyle::Legacy => function
                .call::<()>((EngineHandle, EntityProxy::live(entity), EntityProxy::live(other))),
        },
    );
    finish_debug_callback(state, debugger, &result);
    result.map_err(|error| error.to_string())?;
    Ok(commands)
}
