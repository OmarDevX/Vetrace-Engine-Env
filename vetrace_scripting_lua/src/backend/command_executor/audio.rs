use super::*;

pub(super) fn play_audio(
    engine: &mut Engine,
    spawned: &mut HashMap<u64, Entity>,
    request: u64,
    path: String,
    position: Option<glam::Vec3>,
    volume: f32,
    looping: bool,
) {
    let resolved = engine
        .get_resource::<crate::LuaProjectContext>()
        .ok_or_else(|| "Lua project context is unavailable".to_owned())
        .and_then(|context| context.resolve_existing(&path));
    let Ok(resolved) = resolved else {
        eprintln!("Lua Audio: rejected or missing asset '{path}'");
        return;
    };

    let actor = engine.spawn_actor("Lua Audio").build();
    if let Some(position) = position {
        let _ = actor.insert(
            engine,
            Transform {
                translation: position,
                ..Transform::default()
            },
        );
    }
    let source = AudioSource {
        path: resolved.to_string_lossy().to_string(),
        volume,
        pitch: 1.0,
        looping,
        spatial: position.is_some(),
        play_on_spawn: true,
        state: AudioPlayState::Playing,
        load_mode: if looping {
            AudioLoadMode::Streaming
        } else {
            AudioLoadMode::Static
        },
        auto_despawn: !looping,
        max_distance: 60.0,
    };
    if let Err(error) = actor.insert(engine, source) {
        eprintln!("Lua Audio: failed to create source: {error}");
        actor.despawn(engine);
        return;
    }
    spawned.insert(request, actor.entity());
    remember_entity_handle(engine, request, actor.entity());
}
