use super::*;

pub(super) fn spawn_entity(
    engine: &mut Engine,
    request: u64,
    name: String,
    spawned: &mut HashMap<u64, Entity>,
) {
    let entity = engine.spawn_actor(name).build().entity();
    spawned.insert(request, entity);
    remember_entity_handle(engine, request, entity);
}

pub(super) fn instantiate_scene(
    engine: &mut Engine,
    request: u64,
    path: String,
    spawned: &mut HashMap<u64, Entity>,
) {
    let resolved = engine
        .get_resource::<crate::LuaProjectContext>()
        .ok_or_else(|| "Lua project context is unavailable".to_owned())
        .and_then(|context| context.resolve_existing(&path));
    match resolved.and_then(|resolved_path| {
        let document = load_scene_file(&resolved_path).map_err(|error| error.to_string())?;
        document
            .instantiate_with_assets(engine, &resolved_path)
            .map(|(instance, _)| instance)
            .map_err(|error| error.to_string())
    }) {
        Ok(instance) => {
            if let Some(root) = instance.roots.first().copied() {
                spawned.insert(request, root.entity());
                remember_entity_handle(engine, request, root.entity());
            } else {
                eprintln!("Lua Scene.instantiate: '{path}' contains no root nodes");
            }
        }
        Err(error) => eprintln!("Lua Scene.instantiate failed for '{path}': {error}"),
    }
}

pub(super) fn destroy_entity(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    entity: Entity,
    commands: &mut VecDeque<LuaCommand>,
) {
    if engine.actor(entity).is_none() {
        return;
    }
    let should_destroy_callback = state
        .instances
        .get(&entity)
        .is_some_and(|instance| instance.status == LuaScriptInstanceStatus::Running);
    if should_destroy_callback {
        if let Some(instance) = state.instances.get_mut(&entity) {
            instance.status = LuaScriptInstanceStatus::Disabled;
        }
        match invoke_entity_callback(
            engine,
            state,
            entity,
            EntityCallback::Destroy,
            0.0,
            false,
        ) {
            Ok(nested) => commands.extend(nested),
            Err(message) => {
                let script = state
                    .entity_scripts
                    .get(&entity)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_owned());
                record_error(
                    engine,
                    LuaDiagnosticTarget::Entity { entity, script },
                    "destroy",
                    message,
                );
            }
        }
    }
    state.detach_entity(entity);
    forget_entity_handles(engine, entity);
    if let Some(actor) = engine.actor(entity) {
        actor.despawn(engine);
    }
}

pub(super) fn clear_scene(engine: &mut Engine, state: &mut LuaScriptingState) {
    let entities = state.entity_scripts.keys().copied().collect::<Vec<_>>();
    for entity in entities {
        shutdown_entity_script_inner(engine, state, entity);
        state.detach_entity(entity);
    }
    let world_entities = engine.raw_world().entities().collect::<Vec<_>>();
    for entity in world_entities {
        if let Some(actor) = engine.actor(entity) {
            actor.despawn(engine);
        }
    }
    clear_entity_handles(engine);
}
