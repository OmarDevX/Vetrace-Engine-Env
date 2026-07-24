use super::*;

pub(super) fn add_component(
    engine: &mut Engine,
    spawned: &HashMap<u64, Entity>,
    target: LuaEntityTarget,
    component: String,
    value: Option<vetrace_core::DynamicValue>,
) {
    let Some(entity) = resolve_command_target(engine, target, spawned) else {
        eprintln!("Lua reflection: pending entity was not spawned before adding `{component}`");
        return;
    };
    let Some(actor) = engine.actor(entity) else {
        return;
    };
    if let Err(error) = engine.add_lua_component(actor, &component, value) {
        eprintln!(
            "Lua reflection: failed to add `{component}` to entity {}: {error}",
            entity.0
        );
    }
}

pub(super) fn remove_component(
    engine: &mut Engine,
    spawned: &HashMap<u64, Entity>,
    target: LuaEntityTarget,
    component: String,
) {
    let Some(entity) = resolve_command_target(engine, target, spawned) else {
        return;
    };
    let Some(actor) = engine.actor(entity) else {
        return;
    };
    if let Err(error) = engine.remove_reflected_component(actor, &component) {
        eprintln!(
            "Lua reflection: failed to remove `{component}` from entity {}: {error}",
            entity.0
        );
    }
}
