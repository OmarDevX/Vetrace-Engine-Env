use super::*;

pub(super) fn shutdown_entity_script_inner(engine: &mut Engine, state: &mut LuaScriptingState, entity: Entity) {
    let running = state
        .instances
        .get(&entity)
        .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
    if !running { return; }
    if let Some(instance) = state.instances.get_mut(&entity) {
        instance.status = LuaScriptInstanceStatus::Disabled;
    }
    let script = state
        .entity_scripts
        .get(&entity)
        .cloned()
        .unwrap_or_else(|| "unknown".to_owned());
    match invoke_entity_callback(engine, state, entity, EntityCallback::Destroy, 0.0, false) {
        Ok(commands) => flush_lua_commands(engine, state, commands),
        Err(message) => record_error(
            engine,
            LuaDiagnosticTarget::Entity { entity, script },
            "destroy",
            message,
        ),
    }
}
