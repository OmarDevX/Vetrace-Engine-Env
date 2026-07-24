use super::*;

pub(super) fn synchronize_script_components_inner(engine: &mut Engine, state: &mut LuaScriptingState) {
    let stale = state
        .entity_scripts
        .iter()
        .filter_map(|(entity, active_script)| {
            let stale = match engine.raw_world().get::<ScriptComponent>(*entity) {
                Some(component) => !component.enabled || component.script != *active_script,
                None => true,
            };
            stale.then_some(*entity)
        })
        .collect::<Vec<_>>();

    for entity in stale {
        shutdown_entity_script_inner(engine, state, entity);
        state.detach_entity(entity);
    }
}

pub(super) fn shutdown_autoload_scripts_inner(engine: &mut Engine, state: &mut LuaScriptingState) {
    let names = state.autoload_scripts.clone();
    for name in names {
        let running = state
            .autoload_instances
            .get(&name)
            .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
        if running {
            if let Some(instance) = state.autoload_instances.get_mut(&name) {
                instance.status = LuaScriptInstanceStatus::Disabled;
            }
            match invoke_autoload_callback(
                engine,
                state,
                &name,
                EntityCallback::Destroy,
                0.0,
                false,
            ) {
                Ok(commands) => flush_lua_commands(engine, state, commands),
                Err(message) => record_error(
                    engine,
                    LuaDiagnosticTarget::Autoload(name.clone()),
                    "destroy",
                    message,
                ),
            }
        }
        state.autoload_instances.remove(&name);
        state.started_autoload_scripts.remove(&name);
    }
}

pub(super) fn start_pending_autoload_scripts_inner(engine: &mut Engine, state: &mut LuaScriptingState) {
    let pending = state
        .autoload_scripts
        .iter()
        .filter(|name| !state.started_autoload_scripts.contains(*name))
        .cloned()
        .collect::<Vec<_>>();

    for name in pending {
        if error_budget_exhausted(engine) { break; }
        if !state.scripts.contains_key(&name) { continue; }
        if let Err(error) = state.create_autoload_instance(&name) {
            record_error(engine, LuaDiagnosticTarget::Autoload(name.clone()), "instance", error.to_string());
            state.started_autoload_scripts.insert(name);
            continue;
        }
        let result = invoke_autoload_callback(engine, state, &name, EntityCallback::Ready, 0.0, false);
        match result {
            Ok(commands) => {
                if let Some(instance) = state.autoload_instances.get_mut(&name) {
                    instance.status = LuaScriptInstanceStatus::Running;
                }
                flush_lua_commands(engine, state, commands);
            }
            Err(message) => {
                mark_autoload_failed(state, &name, &message);
                record_error(engine, LuaDiagnosticTarget::Autoload(name.clone()), "ready", message);
            }
        }
        state.started_autoload_scripts.insert(name);
    }
}

pub(super) fn start_pending_scripts_inner(engine: &mut Engine, state: &mut LuaScriptingState) {
    let pending = engine
        .raw_world()
        .query::<ScriptComponent>()
        .into_iter()
        .filter_map(|(entity, component)| {
            let already_started = state.started_scripts.contains(&entity)
                || state.instances.contains_key(&entity);
            (component.enabled && !already_started)
                .then_some((entity, component.script.clone(), component.properties.clone()))
        })
        .collect::<Vec<_>>();

    for (entity, name, properties) in pending {
        if error_budget_exhausted(engine) { break; }
        if !state.scripts.contains_key(&name) {
            let load_result = engine
                .get_resource::<crate::LuaProjectContext>()
                .ok_or_else(|| "Lua project context is unavailable".to_owned())
                .and_then(|context| context.resolve_existing(&name))
                .and_then(|path| {
                    state
                        .load_script_from_file_as(path, name.clone())
                        .map_err(|error| error.to_string())
                });
            if let Err(message) = load_result {
                record_error(
                    engine,
                    LuaDiagnosticTarget::Entity { entity, script: name.clone() },
                    "instance",
                    message,
                );
                state.started_scripts.insert(entity);
                continue;
            }
        }
        state.attach_loaded_script(entity, name.clone());
        if let Err(error) = state.create_entity_instance(entity, &name, &properties) {
            record_error(
                engine,
                LuaDiagnosticTarget::Entity { entity, script: name.clone() },
                "instance",
                error.to_string(),
            );
            state.started_scripts.insert(entity);
            continue;
        }

        let result = invoke_entity_callback(engine, state, entity, EntityCallback::Ready, 0.0, false);
        match result {
            Ok(commands) => {
                if let Some(instance) = state.instances.get_mut(&entity) {
                    instance.status = LuaScriptInstanceStatus::Running;
                }
                flush_lua_commands(engine, state, commands);
            }
            Err(message) => {
                mark_entity_failed(state, entity, &message);
                record_error(
                    engine,
                    LuaDiagnosticTarget::Entity { entity, script: name.clone() },
                    "ready",
                    message,
                );
            }
        }
        if engine.actor(entity).is_some() {
            state.started_scripts.insert(entity);
        } else {
            state.detach_entity(entity);
        }
    }
}

pub(super) fn update_autoload_scripts_inner(engine: &mut Engine, state: &mut LuaScriptingState, dt: f32) {
    let scripts = state.autoload_scripts.clone();
    for name in scripts {
        if error_budget_exhausted(engine) { break; }
        let running = state
            .autoload_instances
            .get(&name)
            .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
        if !running { continue; }
        match invoke_autoload_callback(engine, state, &name, EntityCallback::Update, dt, false) {
            Ok(commands) => flush_lua_commands(engine, state, commands),
            Err(message) => {
                mark_autoload_failed(state, &name, &message);
                record_error(engine, LuaDiagnosticTarget::Autoload(name.clone()), "update", message);
            }
        }
    }
}

pub(super) fn update_autoload_fixed_scripts_inner(engine: &mut Engine, state: &mut LuaScriptingState, dt: f32) {
    let scripts = state.autoload_scripts.clone();
    for name in scripts {
        if error_budget_exhausted(engine) { break; }
        let running = state
            .autoload_instances
            .get(&name)
            .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
        if !running { continue; }
        match invoke_autoload_callback(engine, state, &name, EntityCallback::FixedUpdate, dt, true) {
            Ok(commands) => flush_lua_commands(engine, state, commands),
            Err(message) => {
                mark_autoload_failed(state, &name, &message);
                record_error(
                    engine,
                    LuaDiagnosticTarget::Autoload(name.clone()),
                    "fixed_update",
                    message,
                );
            }
        }
    }
}

pub(super) fn update_scripts_inner(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    dt: f32,
    fixed_update: bool,
) {
    let entities = state.entity_scripts.keys().copied().collect::<Vec<_>>();
    let callback = if fixed_update { EntityCallback::FixedUpdate } else { EntityCallback::Update };
    for entity in entities {
        if error_budget_exhausted(engine) { break; }
        let enabled = engine
            .raw_world()
            .get::<ScriptComponent>(entity)
            .is_some_and(|component| component.enabled);
        let running = state
            .instances
            .get(&entity)
            .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
        if !enabled || !running { continue; }
        let script = state
            .entity_scripts
            .get(&entity)
            .cloned()
            .unwrap_or_else(|| "unknown".to_owned());
        match invoke_entity_callback(engine, state, entity, callback, dt, fixed_update) {
            Ok(commands) => flush_lua_commands(engine, state, commands),
            Err(message) => {
                mark_entity_failed(state, entity, &message);
                record_error(
                    engine,
                    LuaDiagnosticTarget::Entity { entity, script },
                    callback.name(),
                    message,
                );
            }
        }
    }
}
