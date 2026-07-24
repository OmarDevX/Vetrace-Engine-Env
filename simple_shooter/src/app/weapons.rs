use super::*;

pub(crate) const DEFAULT_WEAPON_DEFINITION: &str = "weapons/default_weapon.json";

pub(crate) fn load_active_weapon(engine: &mut Engine) {
    let default_path = resolve_simple_shooter_asset_path(DEFAULT_WEAPON_DEFINITION);
    let directory = default_path.parent().map(std::path::Path::to_path_buf).unwrap_or_default();
    let mut paths = std::fs::read_dir(&directory).ok().into_iter().flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut definitions = Vec::new();
    for path in paths {
        match std::fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))
            .and_then(|json| serde_json::from_str::<WeaponDefinition>(&json).map_err(|error| format!("invalid JSON in {}: {error}", path.display())))
        {
            Ok(definition) => definitions.push(definition.sanitized()),
            Err(error) => eprintln!("Simple Shooter weapon: {error}; skipping definition"),
        }
    }
    if definitions.is_empty() {
        eprintln!("Simple Shooter weapon: no valid definitions in {}; using built-in defaults", directory.display());
        definitions.push(WeaponDefinition::default());
    }
    engine.insert_resource(WeaponRegistry::from_definitions(definitions));
}

pub(crate) fn weapon_registry(engine: &Engine) -> Option<&WeaponRegistry> { engine.get_resource::<WeaponRegistry>() }

pub(crate) fn weapon_definition(engine: &Engine, weapon_id: &str) -> WeaponDefinition {
    weapon_registry(engine).map(|registry| registry.get_or_default(weapon_id).clone()).unwrap_or_default()
}

pub(crate) fn equipped_weapon_id(engine: &Engine, owner: Actor) -> String {
    owner.get_component::<EquippedWeapon>(engine)
        .map(|equipped| equipped.weapon_id.clone())
        .unwrap_or_else(|| DEFAULT_WEAPON_ID.to_string())
}

pub(crate) fn weapon_gameplay_fingerprint(engine: &Engine) -> u64 {
    weapon_registry(engine).map(WeaponRegistry::gameplay_fingerprint).unwrap_or(0)
}

pub(crate) fn equip_weapon(engine: &mut Engine, owner: Actor, weapon_id: &str) -> Result<(), String> {
    if !owner.is_alive(engine) || !owner.has::<ShooterPlayer>(engine) {
        return Err("cannot equip a dead or non-player actor".to_string());
    }
    if weapon_registry(engine).and_then(|registry| registry.get(weapon_id)).is_none() {
        return Err(format!("unknown weapon id `{weapon_id}`"));
    }
    owner.insert(engine, EquippedWeapon { weapon_id: weapon_id.to_string(), cooldown_remaining: 0.0 })
        .map_err(|error| error.to_string())?;
    let old_visuals = engine.actors_with::<WeaponAttachment>()
        .into_iter()
        .filter_map(|(actor, attachment)| (attachment.owner == owner).then_some(actor))
        .collect::<Vec<_>>();
    for visual in old_visuals { visual.despawn(engine); }
    Ok(())
}

pub(crate) fn weapon_rotation(yaw: f32, pitch: f32, extra_degrees: [f32; 3]) -> Quat {
    let base = Quat::from_rotation_y(-yaw) * Quat::from_rotation_x(pitch);
    let extra = vec3(extra_degrees) * (std::f32::consts::PI / 180.0);
    base * Quat::from_euler(glam::EulerRot::XYZ, extra.x, extra.y, extra.z)
}

pub(crate) fn weapon_visual_transform(
    engine: &Engine,
    owner: Actor,
    config: &WeaponDefinition,
    presentation: WeaponPresentation,
) -> Option<Transform> {
    let player = owner.get_component::<ShooterPlayer>(engine)?;
    let center = owner.get_component::<Transform>(engine)?.translation;
    let rotation = weapon_rotation(player.yaw, player.pitch, config.attachment.rotation_degrees);
    Some(Transform {
        translation: player_eye_position(center) + rotation * vec3(match presentation {
            WeaponPresentation::FirstPerson => config.attachment.first_person_position,
            WeaponPresentation::World => config.attachment.position,
        }),
        rotation,
        scale: Vec3::ONE,
    })
}

pub(crate) fn weapon_muzzle(engine: &Engine, owner: Actor, config: &WeaponDefinition) -> Option<(Vec3, Vec3)> {
    let transform = weapon_visual_transform(engine, owner, config, WeaponPresentation::World)?;
    let muzzle = transform.translation + transform.rotation * vec3(config.attachment.muzzle);
    Some((muzzle, transform.rotation * Vec3::NEG_Z))
}

pub(crate) fn find_player_weapon(engine: &Engine, owner: Actor, presentation: WeaponPresentation) -> Option<Actor> {
    engine.actors_with::<WeaponAttachment>()
        .into_iter()
        .find_map(|(actor, attachment)| (attachment.owner == owner && attachment.presentation == presentation).then_some(actor))
}

pub(crate) fn gun_material(config: &WeaponModelConfig, color: [f32; 3]) -> Material {
    Material {
        base_color: vec3(color),
        roughness: config.roughness.clamp(0.0, 1.0),
        metallic: config.metallic.clamp(0.0, 1.0),
        ..Material::default()
    }
}

pub(crate) fn spawn_weapon_box(engine: &mut Engine, root: Actor, name: &str, position: Vec3, size: [f32; 3], material: Material) {
    engine.spawn_actor(name)
        .with(WeaponPart)
        .with(Transform { translation: position, ..Transform::default() })
        .with(Shape { primitive: PrimitiveShape::Cube, size: vec3(size) })
        .with(material)
        .with(Renderable { visible: true, ..Renderable::default() })
        .with(BakedLightProbeReceiver::default())
        .child_of(root).expect("weapon root must be alive")
        .source("simple_shooter/weapon")
        .build();
}

pub(crate) fn spawn_procedural_weapon(engine: &mut Engine, root: Actor, config: &WeaponModelConfig) {
    let body = gun_material(config, config.body_color);
    let accent = gun_material(config, config.accent_color);
    spawn_weapon_box(engine, root, "Gun Body", Vec3::new(0.0, 0.0, -0.25), config.body_size, body.clone());
    spawn_weapon_box(engine, root, "Gun Barrel", Vec3::new(0.0, 0.015, -0.69), config.barrel_size, accent.clone());
    spawn_weapon_box(engine, root, "Gun Stock", Vec3::new(0.0, -0.015, 0.14), config.stock_size, accent.clone());
    spawn_weapon_box(engine, root, "Gun Grip", Vec3::new(0.0, -0.19, -0.10), config.grip_size, body);
}

#[cfg(feature = "gltf")]
pub(crate) fn try_spawn_weapon_model(engine: &mut Engine, root: Actor, config: &WeaponModelConfig) -> bool {
    let Some(path) = config.path.as_deref() else { return false; };
    let path = resolve_simple_shooter_asset_path(path);
    match vetrace_render::load_gltf_actor(engine, &path) {
        Ok(model) => {
            model.insert(engine, WeaponPart).expect("loaded weapon model must be alive");
            model.insert(engine, Renderable { visible: true, ..Renderable::default() }).ok();
            model.set_parent(engine, root).expect("weapon root must be alive");
            model.insert(engine, Transform::default()).expect("loaded weapon model must be alive");
            let mut probe_receivers = weapon_descendants(engine, model);
            probe_receivers.push(model);
            for part in probe_receivers {
                part.insert(engine, BakedLightProbeReceiver::default()).ok();
            }
            true
        }
        Err(error) => {
            eprintln!("Simple Shooter weapon: failed to load model `{}`: {error:#}; using procedural gun", path.display());
            false
        }
    }
}

#[cfg(not(feature = "gltf"))]
pub(crate) fn try_spawn_weapon_model(_engine: &mut Engine, _root: Actor, config: &WeaponModelConfig) -> bool {
    if let Some(path) = config.path.as_deref() {
        eprintln!("Simple Shooter weapon: model `{path}` requires --features gltf; using procedural gun");
    }
    false
}

pub(crate) fn spawn_player_weapon(engine: &mut Engine, owner: Actor, presentation: WeaponPresentation) -> Actor {
    let weapon_id = equipped_weapon_id(engine, owner);
    let config = weapon_definition(engine, &weapon_id);
    let transform = weapon_visual_transform(engine, owner, &config, presentation).unwrap_or_default();
    let root = engine.spawn_actor(format!("{}: {}", config.name, owner.entity().index()))
        .with(WeaponAttachment { owner, weapon_id: weapon_id.clone(), presentation })
        .with(PlayerVisualOwner {
            owner,
            kind: match presentation {
                WeaponPresentation::FirstPerson => PlayerVisualKind::FirstPersonWeapon,
                WeaponPresentation::World => PlayerVisualKind::WorldWeapon,
            },
        })
        .with(transform)
        .source("simple_shooter/weapon")
        .build();
    let model_rotation = vec3(config.model.rotation_degrees) * (std::f32::consts::PI / 180.0);
    let visual_root = engine.spawn_actor("Gun Model Root")
        .with(WeaponPart)
        .with(Transform {
            translation: vec3(config.model.position),
            rotation: Quat::from_euler(glam::EulerRot::XYZ, model_rotation.x, model_rotation.y, model_rotation.z),
            scale: vec3(config.model.scale),
        })
        .child_of(root).expect("weapon attachment must be alive")
        .source("simple_shooter/weapon")
        .build();
    if !try_spawn_weapon_model(engine, visual_root, &config.model) {
        spawn_procedural_weapon(engine, visual_root, &config.model);
    }
    root
}

pub(crate) fn update_weapon_visuals(engine: &mut Engine) {
    if !engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) {
        despawn_all_player_weapons(engine);
        return;
    }
    let attachments = engine.actors_with::<WeaponAttachment>()
        .into_iter().map(|(actor, attachment)| (actor, attachment.clone())).collect::<Vec<_>>();
    for (root, attachment) in attachments {
        let owner = attachment.owner;
        if !owner.is_alive(engine) || !owner.has::<ShooterPlayer>(engine) {
            root.despawn(engine);
            continue;
        }
        let equipped_id = equipped_weapon_id(engine, owner);
        if equipped_id != attachment.weapon_id {
            root.despawn(engine);
            continue;
        }
        let config = weapon_definition(engine, &equipped_id);
        if let Some(transform) = weapon_visual_transform(engine, owner, &config, attachment.presentation) {
            root.insert(engine, transform).expect("weapon root must be alive");
        }
        let alive = owner.get_component::<ShooterPlayer>(engine).map(|player| player.alive).unwrap_or(false);
        let owner_visible = owner.get_component::<Renderable>(engine).map(|renderable| renderable.visible).unwrap_or(true);
        let local = owner.has::<LocalPlayer>(engine);
        let free_flight = owner.get_component::<FreeFlightController>(engine).map(|controller| controller.enabled).unwrap_or(false);
        let visible = alive && match attachment.presentation {
            WeaponPresentation::FirstPerson => local && !free_flight,
            WeaponPresentation::World => owner_visible && (!local || free_flight),
        };
        let descendants = weapon_descendants(engine, root);
        for part in descendants {
            if let Some(renderable) = part.get_component_mut::<Renderable>(engine) { renderable.visible = visible; }
        }
    }
    // Repair attachments if a scene/editor operation removed just the gun.
    let owners = engine.actors_with::<ShooterPlayer>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for owner in owners {
        if find_player_weapon(engine, owner, WeaponPresentation::World).is_none() {
            spawn_player_weapon(engine, owner, WeaponPresentation::World);
        }
        if owner.has::<LocalPlayer>(engine) && find_player_weapon(engine, owner, WeaponPresentation::FirstPerson).is_none() {
            spawn_player_weapon(engine, owner, WeaponPresentation::FirstPerson);
        }
    }
}

pub(crate) fn cleanup_orphan_player_visuals(engine: &mut Engine) {
    let visuals = engine.actors_with::<PlayerVisualOwner>()
        .into_iter().map(|(actor, owner)| (actor, owner.owner)).collect::<Vec<_>>();
    for (visual, owner) in visuals {
        if !owner.is_alive(engine) || !owner.has::<ShooterPlayer>(engine) { visual.despawn(engine); }
    }
}

pub(crate) fn weapon_descendants(engine: &Engine, root: Actor) -> Vec<Actor> {
    let mut result = Vec::new();
    let mut pending = root.children(engine);
    while let Some(actor) = pending.pop() {
        pending.extend(actor.children(engine));
        result.push(actor);
    }
    result
}

pub(crate) fn despawn_all_player_weapons(engine: &mut Engine) {
    let weapons = engine.actors_with::<WeaponAttachment>()
        .into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for weapon in weapons { weapon.despawn(engine); }
}

pub(crate) fn set_all_player_weapons_visible(engine: &mut Engine, visible: bool) {
    let roots = engine.actors_with::<WeaponAttachment>()
        .into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for root in roots {
        for part in weapon_descendants(engine, root) {
            if let Some(renderable) = part.get_component_mut::<Renderable>(engine) {
                renderable.visible = visible;
            }
        }
    }
}

pub(crate) fn spawn_muzzle_flash(engine: &mut Engine, weapon_id: &str, from: Vec3, to: Vec3) {
    let config = weapon_definition(engine, weapon_id);
    if !config.muzzle_flash.enabled { return; }
    let direction = safe_normalize(to - from, Vec3::NEG_Z);
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, direction);
    let color = vec3(config.muzzle_flash.color);
    engine.spawn_actor("Muzzle Flash")
        .with(MuzzleFlash { ttl: config.muzzle_flash.lifetime_seconds })
        .with(Transform { translation: from + direction * config.muzzle_flash.size * 0.35, rotation, scale: Vec3::ONE })
        .with(Shape { primitive: PrimitiveShape::Cube, size: Vec3::new(config.muzzle_flash.size, config.muzzle_flash.size, config.muzzle_flash.size * 1.8) })
        .with(Material { base_color: color, emissive: color * config.muzzle_flash.emissive_intensity, ..Material::default() })
        .with(EmissiveLightEmitter {
            intensity: config.muzzle_flash.light_intensity,
            range: config.muzzle_flash.light_range,
            ..EmissiveLightEmitter::default()
        })
        .with(Renderable { visible: true, ..Renderable::default() })
        .source("simple_shooter/weapon_effect")
        .build();
}

pub(crate) fn present_shot_result(engine: &mut Engine, result: &ShotResult) {
    if !engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) { return; }
    spawn_bullet_trail(engine, &result.weapon_id, result.muzzle, result.endpoint);
    spawn_muzzle_flash(engine, &result.weapon_id, result.muzzle, result.endpoint);
    spawn_shoot_sound(engine, &result.weapon_id, result.muzzle);
}

pub(crate) fn present_pending_shots(engine: &mut Engine) {
    for result in engine.drain_events::<ShotResult>() { present_shot_result(engine, &result); }
}

pub(crate) fn update_muzzle_flashes(engine: &mut Engine, dt: f32) {
    let mut expired = Vec::new();
    engine.query_mut::<MuzzleFlash>().for_each(|actor, flash| {
        flash.ttl -= dt;
        if flash.ttl <= 0.0 { expired.push(actor); }
    });
    engine.defer(|commands| for actor in expired { commands.despawn(actor); });
}
