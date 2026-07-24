use super::*;

pub(crate) fn update_respawns(engine: &mut Engine, dt: f32) {
    let dead: Vec<(Actor, u64)> = engine.actors_with::<ShooterPlayer>()
        .into_iter()
        .filter_map(|(actor, player)| (!player.alive).then_some((actor, player.id)))
        .collect();

    for (actor, id) in dead {
        let mut respawn = false;
        if let Some(player) = actor.get_component_mut::<ShooterPlayer>(engine) {
            player.respawn_timer -= dt;
            if player.respawn_timer <= 0.0 {
                player.health = MAX_HEALTH;
                player.alive = true;
                player.life_damage_by_attacker.clear();
                respawn = true;
            }
        }
        let spawn_position = spawn_position_for_active_map(engine, id);
        if respawn {
            teleport_player_body(engine, actor, spawn_position, Vec3::ZERO);
            set_player_visible(engine, actor, true);
        } else {
            // Keep dead bodies pinned to spawn for the whole respawn delay, not
            // only on the death frame. This prevents Rapier gravity from moving
            // the hidden body between snapshots.
            teleport_player_body(engine, actor, spawn_position, Vec3::ZERO);
            set_player_visible(engine, actor, false);
        }
    }
}


pub(crate) fn teleport_player_body(engine: &mut Engine, actor: Actor, position: Vec3, linear_velocity: Vec3) {
    // Only update ECS state here. `RapierPhysicsPlugin` now owns the bridge from
    // Transform/Velocity into Rapier and detects this external Transform change
    // before the next physics step. That keeps gameplay/networking/editor code
    // from manually syncing two separate worlds.
    if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
        transform.translation = position;
    }
    if let Some(velocity) = actor.get_component_mut::<Velocity>(engine) {
        velocity.linear = linear_velocity;
    }
    if let Some(angular_velocity) = actor.get_component_mut::<AngularVelocity>(engine) {
        angular_velocity.angular = Vec3::ZERO;
    }
    if let Some(body) = actor.get_component_mut::<CharacterBody3D>(engine) {
        body.desired_velocity = Vec3::ZERO;
        body.jump_requested = false;
    }
    if let Some(state) = actor.get_component_mut::<CharacterControllerState>(engine) {
        *state = CharacterControllerState::default();
    }
}
