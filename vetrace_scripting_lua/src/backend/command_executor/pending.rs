use super::*;

pub(super) fn set_pending_name(
    engine: &mut Engine,
    spawned: &HashMap<u64, Entity>,
    request: u64,
    name: String,
) {
    if let Some(actor) = resolve_request(engine, spawned, request)
        .and_then(|entity| engine.actor(entity))
    {
        let _ = actor.set_name(engine, name);
    }
}

pub(super) fn add_pending_tag(
    engine: &mut Engine,
    spawned: &HashMap<u64, Entity>,
    request: u64,
    tag: String,
) {
    if let Some(actor) = resolve_request(engine, spawned, request)
        .and_then(|entity| engine.actor(entity))
    {
        let _ = actor.add_tag(engine, tag);
    }
}
