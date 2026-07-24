use super::*;

pub(crate) const KILLCAM_FOLLOW_SPEED: f32 = 6.5;
pub(crate) const KILLCAM_DISTANCE: f32 = 4.5;
pub(crate) const KILLCAM_HEIGHT: f32 = 1.35;

pub(crate) fn update_killcam_camera(engine: &mut Engine, victim: &ShooterPlayer, dt: f32) {
    let current_camera = engine.get_resource::<Camera>().cloned().unwrap_or_default();
    let desired = victim.last_killer_id
        .and_then(|id| find_player_actor(engine, id))
        .and_then(|actor| {
            let transform = actor.get_component::<Transform>(engine)?;
            let player = actor.get_component::<ShooterPlayer>(engine)?;
            let forward = forward_from_angles(player.yaw, 0.0);
            let focus = player_eye_position(transform.translation);
            Some((focus - forward * KILLCAM_DISTANCE + Vec3::Y * KILLCAM_HEIGHT, focus + forward * 1.8))
        })
        .unwrap_or((current_camera.position, current_camera.target));

    let new_death = engine.get_resource::<ShooterKillcamState>()
        .map(|state| state.death_number != Some(victim.deaths))
        .unwrap_or(true);
    if new_death { spawn_killcam_hud(engine, victim); }

    let (position, target) = {
        let state = engine.get_resource_mut::<ShooterKillcamState>().expect("killcam state is initialized during setup");
        if new_death {
            state.death_number = Some(victim.deaths);
            state.camera_position = current_camera.position;
            state.camera_target = current_camera.target;
        }
        let alpha = 1.0 - (-KILLCAM_FOLLOW_SPEED * dt.max(0.0)).exp();
        state.camera_position = state.camera_position.lerp(desired.0, alpha);
        state.camera_target = state.camera_target.lerp(desired.1, alpha);
        (state.camera_position, state.camera_target)
    };
    if let Some(camera) = engine.get_resource_mut::<Camera>() {
        camera.position = position;
        camera.target = target;
        camera.up = Vec3::Y;
        camera.fov_y_radians = 68.0_f32.to_radians();
    }
}

pub(crate) fn spawn_killcam_hud(engine: &mut Engine, victim: &ShooterPlayer) {
    despawn_killcam_widgets(engine);
    let killer_name = if victim.last_killer_name.is_empty() { "Unknown" } else { &victim.last_killer_name };
    engine.spawn_actor("Killcam panel")
        .with(KillcamWidget)
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.0), offset_px: Vec2::new(0.0, 72.0), size_px: Vec2::new(440.0, 86.0), z_order: 640 })
        .with(vetrace_ui::UIPanel { size: Vec2::new(440.0, 86.0), background: Vec3::new(0.09, 0.015, 0.02), alpha: 0.9, anchor: Anchor::Center })
        .with(vetrace_ui::UIVisualStyle::rounded(12.0).with_border(1.0, Vec3::new(0.9, 0.16, 0.18), 0.9))
        .build();
    engine.spawn_actor("Killcam label")
        .with(KillcamWidget)
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.0), offset_px: Vec2::new(0.0, 72.0), size_px: Vec2::new(410.0, 72.0), z_order: 641 })
        .with(UILabel {
            text: format!("KILLED BY  {}\nDAMAGE DEALT TO YOU  {}", killer_name, victim.last_kill_damage),
            font_size: 18.0,
            color: Vec3::new(1.0, 0.9, 0.9),
            anchor: Anchor::Center,
            align: TextAlign::Center,
        })
        .build();
}

pub(crate) fn clear_killcam(engine: &mut Engine) {
    let active = engine.get_resource::<ShooterKillcamState>().map(|state| state.death_number.is_some()).unwrap_or(false);
    if !active && engine.actors_with::<KillcamWidget>().is_empty() { return; }
    despawn_killcam_widgets(engine);
    if let Some(state) = engine.get_resource_mut::<ShooterKillcamState>() { *state = ShooterKillcamState::default(); }
}

pub(crate) fn despawn_killcam_widgets(engine: &mut Engine) {
    let widgets = engine.actors_with::<KillcamWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in widgets { actor.despawn(engine); }
}
