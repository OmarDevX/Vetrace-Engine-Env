use super::*;

#[derive(Clone, Copy)]
pub(crate) struct ShooterBotDifficultyProfile {
    repath_interval: f32,
    waypoint_reached_distance: f32,
    stop_distance: f32,
    reaction_delay: f32,
    fire_interval: f32,
    aim_error_radians: f32,
    shoot_range: f32,
    movement_scale: f32,
}

pub(crate) fn shooter_bot_difficulty_profile(difficulty: BotDifficulty) -> ShooterBotDifficultyProfile {
    match difficulty {
        BotDifficulty::Easy => ShooterBotDifficultyProfile {
            repath_interval: 0.85,
            waypoint_reached_distance: 1.0,
            stop_distance: 6.0,
            reaction_delay: 0.70,
            fire_interval: 1.15,
            aim_error_radians: 0.11,
            shoot_range: 32.0,
            movement_scale: 0.70,
        },
        BotDifficulty::Normal => ShooterBotDifficultyProfile {
            repath_interval: 0.55,
            waypoint_reached_distance: 0.9,
            stop_distance: 5.0,
            reaction_delay: 0.32,
            fire_interval: 0.68,
            aim_error_radians: 0.055,
            shoot_range: 45.0,
            movement_scale: 0.85,
        },
        BotDifficulty::Hard => ShooterBotDifficultyProfile {
            repath_interval: 0.32,
            waypoint_reached_distance: 0.75,
            stop_distance: 4.0,
            reaction_delay: 0.10,
            fire_interval: 0.38,
            aim_error_radians: 0.016,
            shoot_range: SHOOT_RANGE,
            movement_scale: 1.0,
        },
    }
}

pub(crate) fn bot_combat_input(engine: &mut Engine, bot_id: u64, dt: f32) -> ShooterInput {
    let Some(bot_actor) = find_player_actor(engine, bot_id) else { return ShooterInput::default(); };
    let bot_alive = bot_actor.get_component::<ShooterPlayer>(engine).map(|player| player.alive).unwrap_or(false);
    let Some(bot_position) = bot_actor.get_component::<Transform>(engine).map(|transform| transform.translation) else { return ShooterInput::default(); };
    if !bot_alive {
        let _ = bot_actor.insert(engine, ShooterBotNavigation::default());
        return idle_input_for_player(engine, bot_id);
    }

    let target = engine.actors_with::<ShooterPlayer>().into_iter().filter_map(|(actor, player)| {
        if player.id == bot_id || !player.alive { return None; }
        let position = actor.get_component::<Transform>(engine)?.translation;
        Some((player.id, position, bot_position.distance_squared(position)))
    }).min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((target_id, target_position, distance_squared)) = target else { return ShooterInput::default(); };

    let difficulty = engine.get_resource::<ShooterSession>().map(|session| session.rules.bot_difficulty).unwrap_or_default();
    let profile = shooter_bot_difficulty_profile(difficulty);
    let weapon_id = equipped_weapon_id(engine, bot_actor);
    let weapon_range = weapon_definition(engine, &weapon_id).gameplay.range;
    let mut navigation = bot_actor.get_component::<ShooterBotNavigation>(engine).cloned().unwrap_or_default();
    navigation.repath_timer -= dt.max(0.0);
    navigation.reaction_timer = (navigation.reaction_timer - dt.max(0.0)).max(0.0);
    navigation.fire_timer = (navigation.fire_timer - dt.max(0.0)).max(0.0);
    navigation.aim_phase += dt.max(0.0) * 1.7;
    let target_changed = navigation.target_id != Some(target_id);
    if target_changed || navigation.repath_timer <= 0.0 || navigation.waypoint_index >= navigation.path.len() {
        navigation.target_id = Some(target_id);
        navigation.repath_timer = profile.repath_interval;
        navigation.waypoint_index = 0;
        navigation.path = engine.get_resource::<PathfindingWorld>()
            .and_then(|world| world.find_path(bot_position, target_position))
            .map(|path| path.points)
            .unwrap_or_else(|| vec![target_position]);
        if target_changed { navigation.reaction_timer = profile.reaction_delay; }
    }

    while navigation.waypoint_index < navigation.path.len() {
        let waypoint = navigation.path[navigation.waypoint_index];
        let planar_distance = Vec2::new(waypoint.x - bot_position.x, waypoint.z - bot_position.z).length();
        if planar_distance > profile.waypoint_reached_distance { break; }
        navigation.waypoint_index += 1;
    }
    let waypoint = navigation.path.get(navigation.waypoint_index).copied().unwrap_or(target_position);
    let mut travel = Vec3::new(waypoint.x - bot_position.x, 0.0, waypoint.z - bot_position.z).normalize_or_zero();
    if distance_squared <= profile.stop_distance * profile.stop_distance { travel = Vec3::ZERO; }

    let aim_origin = player_eye_position(bot_position);
    let aim_target = player_eye_position(target_position);
    let exact_aim = (aim_target - aim_origin).normalize_or_zero();
    let horizontal = Vec2::new(exact_aim.x, exact_aim.z).length().max(0.0001);
    let exact_yaw = exact_aim.x.atan2(-exact_aim.z);
    let exact_pitch = exact_aim.y.atan2(horizontal);
    let bot_phase = bot_id as f32 * 0.731 + navigation.aim_phase;
    let yaw = exact_yaw + bot_phase.sin() * profile.aim_error_radians;
    let pitch = exact_pitch + (bot_phase * 0.83 + 1.4).cos() * profile.aim_error_radians * 0.65;
    let forward = forward_from_angles(yaw, 0.0);
    let right = right_from_yaw(yaw);
    let movement = Vec2::new(travel.dot(right), travel.dot(forward)).clamp_length_max(1.0) * profile.movement_scale;
    let target_distance = distance_squared.sqrt();
    let has_line_of_sight = target_distance <= profile.shoot_range.min(weapon_range)
        && first_hitscan_blocker(engine, bot_actor, aim_origin, exact_aim, target_distance).is_none();
    let fire = has_line_of_sight && navigation.reaction_timer <= 0.0 && navigation.fire_timer <= 0.0;
    if fire { navigation.fire_timer = profile.fire_interval; }

    let _ = bot_actor.insert(engine, navigation);
    ShooterInput { movement, yaw, pitch, fire, jump: false }
}

#[cfg(test)]
mod bot_difficulty_tests {
    use super::*;

    #[test]
    fn easier_bots_react_and_fire_more_slowly() {
        let easy = shooter_bot_difficulty_profile(BotDifficulty::Easy);
        let normal = shooter_bot_difficulty_profile(BotDifficulty::Normal);
        let hard = shooter_bot_difficulty_profile(BotDifficulty::Hard);
        assert!(easy.reaction_delay > normal.reaction_delay && normal.reaction_delay > hard.reaction_delay);
        assert!(easy.fire_interval > normal.fire_interval && normal.fire_interval > hard.fire_interval);
        assert!(easy.aim_error_radians > normal.aim_error_radians && normal.aim_error_radians > hard.aim_error_radians);
    }
}
