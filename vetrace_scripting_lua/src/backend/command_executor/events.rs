use super::*;

pub(super) fn emit_event(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    target: Option<Entity>,
    name: String,
    payload: ScriptValue,
    commands: &mut VecDeque<LuaCommand>,
) {
    if target.is_none() {
        let autoloads = state.autoload_scripts.clone();
        for autoload in autoloads {
            match invoke_autoload_event(engine, state, &autoload, &name, &payload) {
                Ok(nested) => commands.extend(nested),
                Err(message) => {
                    mark_autoload_failed(state, &autoload, &message);
                    record_error(
                        engine,
                        LuaDiagnosticTarget::Autoload(autoload),
                        "on_event",
                        message,
                    );
                }
            }
        }
    }

    let targets = match target {
        Some(entity) => vec![entity],
        None => state.instances.keys().copied().collect::<Vec<_>>(),
    };
    for entity in targets {
        match invoke_entity_event(engine, state, entity, &name, &payload) {
            Ok(nested) => commands.extend(nested),
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
    }
}
