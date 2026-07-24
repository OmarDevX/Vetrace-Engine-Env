use super::*;

pub fn attach_loaded_script(engine: &mut Engine, entity: Entity, name: impl Into<String>) {
    ensure_resources(engine);
    let name = name.into();
    with_state(engine, |engine, state| {
        state.attach_loaded_script(entity, name.clone());
        if let Some(component) = engine.raw_world_mut().get_mut::<ScriptComponent>(entity) {
            component.script = name;
        } else {
            engine.raw_world_mut().insert(
                entity,
                ScriptComponent { script: name, ..ScriptComponent::default() },
            );
        }
    });
}

pub fn attach_autoload_script(engine: &mut Engine, name: impl Into<String>) {
    ensure_resources(engine);
    let name = name.into();
    with_state(engine, |_, state| state.attach_autoload_script(name));
}

pub fn detach_script(engine: &mut Engine, entity: Entity) {
    ensure_resources(engine);
    with_state(engine, |engine, state| {
        shutdown_entity_script_inner(engine, state, entity);
        state.detach_entity(entity);
        engine.raw_world_mut().remove::<ScriptComponent>(entity);
    });
}

pub fn shutdown_entity_scripts(engine: &mut Engine, entities: &[Entity]) {
    ensure_resources(engine);
    with_state(engine, |engine, state| {
        for entity in entities.iter().copied() {
            shutdown_entity_script_inner(engine, state, entity);
            state.detach_entity(entity);
        }
    });
}

/// Runs `destroy` for every running autoload and clears their runtime state.
/// Project/runtime hosts should call this once during application shutdown.
pub fn shutdown_autoload_scripts(engine: &mut Engine) {
    ensure_resources(engine);
    with_state(engine, shutdown_autoload_scripts_inner);
}

pub fn load_script_from_file(
    engine: &mut Engine,
    path: impl AsRef<std::path::Path>,
) -> mlua::Result<String> {
    ensure_resources(engine);
    with_state(engine, |_, state| state.load_script_from_file(path))
}

pub fn load_script_from_file_as(
    engine: &mut Engine,
    path: impl AsRef<std::path::Path>,
    name: impl Into<String>,
) -> mlua::Result<String> {
    ensure_resources(engine);
    with_state(engine, |_, state| state.load_script_from_file_as(path, name))
}

pub fn load_scripts_from_dir(
    engine: &mut Engine,
    dir: impl AsRef<std::path::Path>,
) -> mlua::Result<Vec<String>> {
    ensure_resources(engine);
    with_state(engine, |_, state| state.load_scripts_from_dir(dir))
}

/// Replaces a loaded script template and restarts only instances using it.
/// File watching belongs to the editor/runtime host; this operation is the
/// deterministic hot-reload primitive they call after detecting a change.
pub fn reload_script_from_file_as(
    engine: &mut Engine,
    path: impl AsRef<std::path::Path>,
    name: impl Into<String>,
) -> mlua::Result<String> {
    ensure_resources(engine);
    let name = name.into();
    with_state(engine, |engine, state| {
        // Compile the replacement first. If it fails, restore the old template
        // and leave all live instances running unchanged.
        let old_script = state.scripts.remove(&name);
        let loaded = match state.load_script_from_file_as(path, name.clone()) {
            Ok(loaded) => loaded,
            Err(error) => {
                if let Some(old_script) = old_script {
                    state.scripts.insert(name.clone(), old_script);
                }
                return Err(error);
            }
        };
        let new_script = state
            .scripts
            .remove(&name)
            .expect("successfully loaded Lua script must be present");
        if let Some(old_script) = old_script {
            state.scripts.insert(name.clone(), old_script);
        }

        let entities = state
            .entity_scripts
            .iter()
            .filter_map(|(entity, script)| (script == &name).then_some(*entity))
            .collect::<Vec<_>>();
        for entity in entities.iter().copied() {
            shutdown_entity_script_inner(engine, state, entity);
        }

        let autoload_running = state
            .autoload_instances
            .get(&name)
            .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
        if autoload_running {
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

        state.scripts.insert(name.clone(), new_script);
        for entity in entities {
            state.instances.remove(&entity);
            state.started_scripts.remove(&entity);
        }
        if state.autoload_scripts.iter().any(|script| script == &name) {
            state.autoload_instances.remove(&name);
            state.started_autoload_scripts.remove(&name);
        }
        Ok(loaded)
    })
}

pub fn start_pending_autoload_scripts(engine: &mut Engine) {
    ensure_resources(engine);
    with_state(engine, start_pending_autoload_scripts_inner);
}

pub fn start_pending_scripts(engine: &mut Engine) {
    ensure_resources(engine);
    with_state(engine, start_pending_scripts_inner);
}

pub fn update_autoload_scripts(engine: &mut Engine, dt: f32) {
    ensure_resources(engine);
    with_state(engine, |engine, state| update_autoload_scripts_inner(engine, state, dt));
}

pub fn update_scripts(engine: &mut Engine, dt: f32) {
    ensure_resources(engine);
    with_state(engine, |engine, state| {
        synchronize_script_components_inner(engine, state);
        update_scripts_inner(engine, state, dt, false);
    });
}

pub fn fixed_update_scripts(engine: &mut Engine, dt: f32) {
    ensure_resources(engine);
    with_state(engine, |engine, state| {
        update_autoload_fixed_scripts_inner(engine, state, dt);
        update_scripts_inner(engine, state, dt, true);
    });
}

/// Sends a named gameplay event to one running entity script instance.
pub fn dispatch_script_event(
    engine: &mut Engine,
    entity: Entity,
    event: &str,
    payload: ScriptValue,
) {
    ensure_resources(engine);
    with_state(engine, |engine, state| {
        match invoke_entity_event(engine, state, entity, event, &payload) {
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
                    "on_event",
                    message,
                );
            }
        }
    });
}

pub fn dispatch_collision_enter(engine: &mut Engine, entity: Entity, other: Entity) {
    dispatch_collision(engine, entity, other, true);
}

pub fn dispatch_collision_exit(engine: &mut Engine, entity: Entity, other: Entity) {
    dispatch_collision(engine, entity, other, false);
}
