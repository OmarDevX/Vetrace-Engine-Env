use super::*;

pub(crate) fn update_camera(engine: &mut Engine, runtime: &ShooterRuntime, dt: f32) {
    let Some(local_id) = runtime.local_id else { return; };
    let Some(local_actor) = find_player_actor(engine, local_id) else { return; };
    let Some(local_player) = local_actor.get_component::<ShooterPlayer>(engine).cloned() else { return; };

    if !local_player.alive {
        update_killcam_camera(engine, &local_player, dt);
        set_local_player_visible(engine, local_actor, false);
        sync_audio_listener_to_camera(engine);
        return;
    }
    clear_killcam(engine);

    if let Some(free_flight) = local_actor.get_component::<FreeFlightController>(engine).cloned().filter(|controller| controller.enabled) {
        let forward = forward_from_angles(free_flight.yaw, free_flight.pitch);
        if let Some(camera) = engine.get_resource_mut::<Camera>() {
            camera.position = free_flight.position;
            camera.target = free_flight.position + forward;
            camera.up = Vec3::Y;
            camera.fov_y_radians = 75.0_f32.to_radians();
        }
        let alive = local_actor.get_component::<ShooterPlayer>(engine).map(|player| player.alive).unwrap_or(true);
        set_local_player_visible(engine, local_actor, alive);
        sync_audio_listener_to_camera(engine);
        return;
    }

    let Some(transform) = local_actor.get_component::<Transform>(engine).cloned() else { return; };
    let eye = player_eye_position(transform.translation);
    let forward = forward_from_angles(local_player.yaw, local_player.pitch);
    if let Some(camera) = engine.get_resource_mut::<Camera>() {
        camera.position = eye;
        camera.target = eye + forward;
        camera.up = Vec3::Y;
        camera.fov_y_radians = 75.0_f32.to_radians();
    }

    // First-person view: do not draw your own body/outline in front of the camera.
    set_local_player_visible(engine, local_actor, false);
    sync_audio_listener_to_camera(engine);
}

pub(crate) fn update_crosshair_entities(engine: &mut Engine, runtime: &ShooterRuntime) {
    let Some(local_id) = runtime.local_id else { return; };
    let Some(local_actor) = find_player_actor(engine, local_id) else { return; };
    let alive = local_actor.get_component::<ShooterPlayer>(engine).map(|player| player.alive).unwrap_or(false);
    let free_flight = local_actor.get_component::<FreeFlightController>(engine).map(|controller| controller.enabled).unwrap_or(false);
    let visible = alive && !runtime.config.use_scripted_input && !runtime.editor_enabled && !free_flight;

    let parts: Vec<(Actor, bool)> = engine.actors_with::<CrosshairPart>()
        .into_iter()
        .map(|(actor, part)| (actor, part.horizontal))
        .collect();

    for (actor, horizontal) in parts {
        if let Some(rect) = actor.get_component_mut::<ScreenSpaceRect>(engine) {
            rect.anchor = Vec2::new(0.5, 0.5);
            rect.offset_px = Vec2::ZERO;
            rect.size_px = if horizontal { Vec2::new(24.0, 2.0) } else { Vec2::new(2.0, 24.0) };
            rect.z_order = 100;
        }
        if let Some(renderable) = actor.get_component_mut::<Renderable>(engine) {
            renderable.visible = visible;
        }
    }
}

pub(crate) fn set_local_player_visible(engine: &mut Engine, actor: Actor, visible: bool) {
    let is_local = actor.has::<LocalPlayer>(engine);
    if is_local {
        if let Some(renderable) = actor.get_component_mut::<Renderable>(engine) {
            renderable.visible = visible;
        }
        sync_player_outline_style(engine, actor);
    }
}

pub(crate) fn idle_input_for_player(engine: &Engine, player_id: u64) -> ShooterInput {
    let Some(actor) = find_player_actor(engine, player_id) else { return ShooterInput::default(); };
    if let Some(player) = actor.get_component::<ShooterPlayer>(engine) {
        return ShooterInput { yaw: player.yaw, pitch: player.pitch, ..ShooterInput::default() };
    }
    if let Some(controller) = actor.get_component::<FirstPersonController>(engine) {
        return ShooterInput { yaw: controller.yaw, pitch: controller.pitch, ..ShooterInput::default() };
    }
    ShooterInput::default()
}

pub(crate) fn read_first_person_input(engine: &mut Engine, player_id: u64) -> ShooterInput {
    if engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false) {
        return idle_input_for_player(engine, player_id);
    }
    let Some(actor) = find_player_actor(engine, player_id) else { return ShooterInput::default(); };

    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let (mouse_dx, mouse_dy) = input.mouse_delta();
    let mut movement = Vec2::ZERO;
    if input.is_key_down("W") { movement.y += 1.0; }
    if input.is_key_down("S") { movement.y -= 1.0; }
    if input.is_key_down("D") { movement.x += 1.0; }
    if input.is_key_down("A") { movement.x -= 1.0; }
    movement = movement.clamp_length_max(1.0);

    let mut yaw = 0.0;
    let mut pitch = 0.0;
    if let Some(controller) = actor.get_component_mut::<FirstPersonController>(engine) {
        controller.yaw += mouse_dx * controller.mouse_sensitivity;
        controller.pitch = (controller.pitch - mouse_dy * controller.mouse_sensitivity).clamp(-1.35, 1.35);
        yaw = controller.yaw;
        pitch = controller.pitch;
    } else if let Some(player) = actor.get_component::<ShooterPlayer>(engine) {
        yaw = player.yaw;
        pitch = player.pitch;
    }

    ShooterInput {
        movement,
        yaw,
        pitch,
        fire: input.is_mouse_button_down("Left"),
        jump: input.was_key_pressed("Space"),
    }
}

pub(crate) fn set_player_visible(engine: &mut Engine, actor: Actor, visible: bool) {
    if let Some(renderable) = actor.get_component_mut::<Renderable>(engine) {
        renderable.visible = visible;
    }
    sync_player_outline_style(engine, actor);
}

pub(crate) fn update_player_outline_styles(engine: &mut Engine) {
    despawn_orphan_outline_shells(engine);
    let actors = engine.actors_with::<ShooterPlayer>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors {
        sync_player_outline_style(engine, actor);
    }
}

pub(crate) fn sync_player_outline_style(engine: &mut Engine, actor: Actor) {
    if !engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) { return; }
    let Some(alive) = actor.get_component::<ShooterPlayer>(engine).map(|player| player.alive) else { return; };
    let owner_visible = actor.get_component::<Renderable>(engine).map(|renderable| renderable.visible).unwrap_or(true);
    let owner_transform = actor.get_component::<Transform>(engine).cloned().unwrap_or_default();
    let owner_scale = owner_transform.scale;
    let is_local = actor.has::<LocalPlayer>(engine);
    let style = actor.get_component::<ShooterOutlineStyle>(engine).copied().unwrap_or_default();
    let shell_visible = owner_visible && alive && if is_local { style.local_enabled } else { style.remote_enabled };

    let mut shell = find_player_outline_shell(engine, actor);
    if shell.is_none() {
        shell = Some(spawn_player_outline_shell(engine, actor));
    }
    let Some(shell) = shell else { return; };

    let shell_actor = shell;
    shell_actor.insert(engine, owner_transform).expect("outline shell actor must be alive");
    shell_actor.insert(engine, player_outline_shape(style, owner_scale)).expect("outline shell actor must be alive");
    shell_actor
        .insert(engine, Material { base_color: style.color, alpha: 1.0, ..Material::default() })
        .expect("outline shell actor must be alive");
    shell_actor.insert(engine, player_outline_material(style)).expect("outline shell actor must be alive");
    shell_actor
        .insert(engine, Renderable { visible: shell_visible, ..Renderable::default() })
        .expect("outline shell actor must be alive");
}

pub(crate) fn find_player_actor(engine: &Engine, id: u64) -> Option<Actor> {
    engine.actors_with::<ShooterPlayer>().into_iter().find_map(|(actor, player)| (player.id == id).then_some(actor))
}

pub(crate) fn scripted_input(time: f32, id: u64, enabled: bool) -> ShooterInput {
    if !enabled {
        return ShooterInput::default();
    }
    let t = time + id as f32 * 1.37;
    let yaw = t * 0.24;
    let move_x = (t * 0.45).sin() * 0.55;
    let move_y = (t * 0.55).cos() * 0.65;
    ShooterInput {
        movement: Vec2::new(move_x, move_y).clamp_length_max(1.0),
        yaw,
        pitch: 0.03 * (t * 0.5).sin(),
        fire: (time * 1.7 + id as f32).sin() > 0.88,
        jump: false,
    }
}

pub(crate) fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}
