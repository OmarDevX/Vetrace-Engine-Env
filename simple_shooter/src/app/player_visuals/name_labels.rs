use super::*;

pub(crate) fn spawn_player_name_label(engine: &mut Engine, owner: Actor) -> Actor {
    let name = owner.get_component::<ShooterPlayer>(engine)
        .map(|player| player.name.clone())
        .unwrap_or_else(|| "Player".to_string());
    engine
        .spawn_actor(format!("Name Label: {name}"))
        .with(PlayerNameLabel)
        .with(PlayerVisualOwner { owner, kind: PlayerVisualKind::NameLabel })
        .with(player_name_label_transform())
        .with(player_name_label_component(&name))
        .with(player_name_label_world_space(true))
        .child_of(owner)
        .expect("player owner must be alive")
        .build()
}

pub(crate) fn player_name_label_transform() -> Transform {
    Transform {
        translation: Vec3::new(0.0, PLAYER_HEIGHT * 0.5 + 0.48, 0.0),
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    }
}

pub(crate) fn player_name_label_component(name: &str) -> UILabel {
    UILabel {
        text: sanitize_player_name(name),
        font_size: 16.0,
        color: Vec3::ONE,
        anchor: Anchor::BottomCenter,
        align: TextAlign::Center,
    }
}

pub(crate) fn player_name_label_world_space(visible: bool) -> UIWorldSpace {
    UIWorldSpace {
        screen_offset_px: Vec2::new(0.0, -8.0),
        max_distance: 80.0,
        anchor: Anchor::BottomCenter,
        background: Vec3::ZERO,
        background_alpha: 0.58,
        padding_px: Vec2::new(7.0, 3.0),
        visible,
        ..UIWorldSpace::default()
    }
}

pub(crate) fn sanitize_player_name(name: &str) -> String {
    let cleaned = name
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(24)
        .collect::<String>();
    if cleaned.is_empty() { "Player".to_string() } else { cleaned }
}

pub(crate) fn find_player_name_label(engine: &Engine, owner: Actor) -> Option<Actor> {
    engine.actors_with::<Parent>()
        .into_iter()
        .find_map(|(actor, parent)| (parent.0 == owner.entity() && actor.has::<PlayerNameLabel>(engine)).then_some(actor))
}

pub(crate) fn despawn_orphan_name_labels(engine: &mut Engine) {
    let labels = engine.actors_with::<Parent>()
        .into_iter()
        .filter_map(|(actor, parent)| actor.has::<PlayerNameLabel>(engine).then(|| engine.actor(parent.0).map(|parent| (actor, parent))).flatten())
        .collect::<Vec<_>>();
    for (label, owner) in labels {
        if !owner.is_alive(engine) || !owner.has::<ShooterPlayer>(engine) {
            label.despawn(engine);
        }
    }
}

pub(crate) fn update_player_name_labels(engine: &mut Engine) {
    despawn_orphan_name_labels(engine);
    let players = engine.actors_with::<ShooterPlayer>()
        .into_iter()
        .map(|(actor, _)| actor)
        .collect::<Vec<_>>();
    for actor in players {
        sync_player_name_label(engine, actor);
    }
}

pub(crate) fn sync_player_name_label(engine: &mut Engine, owner: Actor) {
    if !engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) { return; }
    let Some(player) = owner.get_component::<ShooterPlayer>(engine).cloned() else { return; };
    let owner_visible = owner.get_component::<Renderable>(engine).map(|renderable| renderable.visible).unwrap_or(true);
    let label_visible = player.alive && owner_visible;

    let mut label = find_player_name_label(engine, owner);
    if label.is_none() {
        label = Some(spawn_player_name_label(engine, owner));
    }
    let Some(label) = label else { return; };

    let label_actor = label;
    let owner_actor = owner;
    if label_actor.parent(engine) != Some(owner_actor) {
        label_actor
            .set_parent(engine, owner_actor)
            .expect("player name label owner must be alive and acyclic");
    }
    label_actor.insert(engine, player_name_label_transform()).expect("player name label must be alive");
    label_actor.insert(engine, player_name_label_component(&player.name)).expect("player name label must be alive");
    label_actor.insert(engine, player_name_label_world_space(label_visible)).expect("player name label must be alive");
}
