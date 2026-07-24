use super::*;

pub(super) fn set_velocity(
    engine: &mut Engine,
    spawned: &HashMap<u64, Entity>,
    target: LuaEntityTarget,
    value: glam::Vec3,
) {
    let Some(actor) = resolve_command_target(engine, target, spawned)
        .and_then(|entity| engine.actor(entity))
    else {
        return;
    };
    if let Err(error) = actor.set_velocity(engine, value) {
        eprintln!(
            "Lua Physics.set_velocity failed for entity {}: {error}",
            actor.entity().0
        );
    }
}

pub(super) fn apply_impulse(
    engine: &mut Engine,
    spawned: &HashMap<u64, Entity>,
    target: LuaEntityTarget,
    value: glam::Vec3,
) {
    let Some(actor) = resolve_command_target(engine, target, spawned)
        .and_then(|entity| engine.actor(entity))
    else {
        return;
    };
    if let Err(error) = actor.apply_impulse(engine, value) {
        eprintln!(
            "Lua Physics.apply_impulse failed for entity {}: {error}",
            actor.entity().0
        );
    }
}
