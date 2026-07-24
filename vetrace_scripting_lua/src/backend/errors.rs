use super::*;

pub(super) fn cleanup_dead_script_instances(engine: &Engine, state: &mut LuaScriptingState) {
    let alive = state
        .entity_scripts
        .keys()
        .copied()
        .filter(|entity| engine.raw_world().is_alive(*entity))
        .collect::<HashSet<_>>();
    state.remove_dead_entities(|entity| alive.contains(&entity));
}

pub(super) fn mark_entity_failed(state: &mut LuaScriptingState, entity: Entity, message: &str) {
    if let Some(instance) = state.instances.get_mut(&entity) {
        instance.status = LuaScriptInstanceStatus::Failed;
        instance.last_error = Some(message.to_owned());
        instance.error_count = instance.error_count.saturating_add(1);
    }
}

pub(super) fn mark_autoload_failed(state: &mut LuaScriptingState, name: &str, message: &str) {
    if let Some(instance) = state.autoload_instances.get_mut(name) {
        instance.status = LuaScriptInstanceStatus::Failed;
        instance.last_error = Some(message.to_owned());
        instance.error_count = instance.error_count.saturating_add(1);
    }
}

pub(super) fn record_error(
    engine: &mut Engine,
    target: LuaDiagnosticTarget,
    callback: &'static str,
    message: String,
) {
    eprintln!("Lua {callback} error ({target:?}): {message}");
    if let Some(errors) = engine.get_resource_mut::<LuaFrameErrors>() {
        errors.0 = errors.0.saturating_add(1);
    }
    if let Some(diagnostics) = engine.get_resource_mut::<LuaDiagnostics>() {
        diagnostics.push(LuaScriptError { target, callback, message });
    }
    if engine
        .get_resource::<LuaRuntimeConfig>()
        .copied()
        .unwrap_or_default()
        .fail_fast
    {
        engine.stop();
    }
}

pub(super) fn error_budget_exhausted(engine: &Engine) -> bool {
    let count = engine.get_resource::<LuaFrameErrors>().copied().unwrap_or_default().0;
    let max = engine
        .get_resource::<LuaRuntimeConfig>()
        .copied()
        .unwrap_or_default()
        .max_errors_per_frame;
    max > 0 && count >= max
}

pub(super) fn with_state<R>(
    operation_engine: &mut Engine,
    operation: impl FnOnce(&mut Engine, &mut LuaScriptingState) -> R,
) -> R {
    let mut state = operation_engine
        .remove_resource::<LuaScriptingState>()
        .unwrap_or_else(LuaScriptingState::new);
    let result = operation(operation_engine, &mut state);
    operation_engine.insert_resource(state);
    result
}

pub(super) fn ensure_resources(engine: &mut Engine) {
    if !engine.contains_resource::<LuaScriptingState>() {
        engine.insert_resource(LuaScriptingState::new());
    }
    if !engine.contains_resource::<LuaDiagnostics>() {
        engine.insert_resource(LuaDiagnostics::default());
    }
    if !engine.contains_resource::<LuaRuntimeConfig>() {
        engine.insert_resource(LuaRuntimeConfig::default());
    }
    if !engine.contains_resource::<LuaFrameErrors>() {
        engine.insert_resource(LuaFrameErrors::default());
    }
}
