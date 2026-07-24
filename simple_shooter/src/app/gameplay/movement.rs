use super::*;

pub(crate) fn apply_input_to_player(engine: &mut Engine, player_id: u64, input: ShooterInput, dt: f32) {
    let Some(actor) = find_player_actor(engine, player_id) else { return; };
    let mod_effects = engine.get_resource::<ShooterModEffects>().copied().unwrap_or_default();
    let session_rules = engine.get_resource::<ShooterSession>().map(|session| session.rules).unwrap_or_default();
    let move_speed = MOVE_SPEED * mod_effects.movement_multiplier * session_rules.move_speed_multiplier;
    let jump_speed = JUMP_SPEED * mod_effects.jump_multiplier * session_rules.jump_multiplier;
    let effective_gravity_scale = session_rules.gravity_scale;

    let mut should_fire = false;
    let mut aim_origin = Vec3::ZERO;
    let mut shot_dir = Vec3::Z;
    let weapon_id = equipped_weapon_id(engine, actor);
    let weapon = weapon_definition(engine, &weapon_id);

    let mut player_is_dead = false;
    if let Some(player) = actor.get_component_mut::<ShooterPlayer>(engine) {
        if !player.alive {
            player_is_dead = true;
        } else {
            player.yaw = input.yaw;
            player.pitch = input.pitch.clamp(-1.35, 1.35);
            shot_dir = forward_from_angles(player.yaw, player.pitch);
        }
    }
    if player_is_dead {
        // Dead players stay pinned at their spawn until respawn. Otherwise gravity
        // can move Rapier's hidden body and the next snapshot/camera update can
        // appear to snap back to the death position or drift downward.
        let spawn = spawn_position_for_active_map(engine, player_id);
        teleport_player_body(engine, actor, spawn, Vec3::ZERO);
        return;
    }

    if let Some(equipped) = actor.get_component_mut::<EquippedWeapon>(engine) {
        equipped.cooldown_remaining = (equipped.cooldown_remaining - dt).max(0.0);
        if input.fire && equipped.cooldown_remaining <= 0.0 {
            equipped.cooldown_remaining = weapon.gameplay.cooldown_seconds;
            should_fire = true;
        }
    }

    if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
        transform.rotation = player_body_rotation(input.yaw);
    }

    let grounded = is_grounded(engine, actor);
    if let Some(controller) = actor.get_component_mut::<FirstPersonController>(engine) {
        controller.yaw = input.yaw;
        controller.pitch = input.pitch;
        controller.grounded = grounded;
    }

    let forward = forward_from_angles(input.yaw, 0.0);
    let right = right_from_yaw(input.yaw);
    let desired = (right * input.movement.x + forward * input.movement.y).clamp_length_max(1.0) * move_speed;

    if let Some(body) = actor.get_component_mut::<CharacterBody3D>(engine) {
        // Game/app owns input intent. The generic physics CharacterBody3D system
        // owns slope projection, jump gating, snap-to-ground and velocity writes.
        body.move_speed = move_speed;
        body.jump_speed = jump_speed;
        body.desired_velocity = desired;
        body.jump_requested = input.jump;
    } else if let Some(velocity) = actor.get_component_mut::<Velocity>(engine) {
        // Fallback for unusual tests where the character body component is not
        // present. Normal Simple Shooter players use CharacterBody3D.
        velocity.linear.x = desired.x;
        velocity.linear.z = desired.z;
        if grounded && input.jump {
            velocity.linear.y = jump_speed;
        } else if grounded && velocity.linear.y < 0.0 {
            velocity.linear.y = 0.0;
        } else {
            velocity.linear.y -= 9.81 * mod_effects.gravity_scale * dt;
        }
    }
    if !grounded && (effective_gravity_scale - 1.0).abs() > f32::EPSILON {
        if let Some(velocity) = actor.get_component_mut::<Velocity>(engine) {
            velocity.linear.y -= 9.81 * (effective_gravity_scale - 1.0) * dt;
        }
    }

    if let Some(transform) = actor.get_component::<Transform>(engine) {
        aim_origin = player_eye_position(transform.translation);
    }

    if should_fire {
        engine.send_event(FireRequest {
            shooter: actor,
            shooter_id: player_id,
            weapon_id,
            aim_origin,
            aim_direction: shot_dir,
        });
    }
}

pub(crate) fn is_grounded(engine: &Engine, actor: Actor) -> bool {
    if let Some(state) = actor.get_component::<CharacterControllerState>(engine) {
        // If a physics plugin is active and has produced controller state, trust
        // it exactly. Falling while still near the ground must not count as
        // grounded, otherwise jump can be repeated in mid-air and vertical
        // velocity gets zeroed.
        return state.grounded;
    }

    let Some(transform) = actor.get_component::<Transform>(engine) else { return false; };
    let vertical_speed = actor.get_component::<Velocity>(engine).map(|v| v.linear.y).unwrap_or(0.0);

    // Fallback for the first frame or headless/no-physics smoke tests. Keep the
    // tolerance tight so it cannot be used as an air-jump detector.
    transform.translation.y <= GROUND_Y + PLAYER_HEIGHT * 0.5 + 0.08 && vertical_speed.abs() <= 0.35
}


pub(crate) fn player_eye_position(center: Vec3) -> Vec3 {
    // Player transforms are body/capsule centers. FPS_EYE_HEIGHT is measured
    // from the feet, so add only the local eye offset from the center. This
    // keeps the camera, crosshair ray and bullet trail inside the visible body
    // instead of floating above the player's head.
    center + Vec3::Y * FPS_EYE_LOCAL_Y
}
