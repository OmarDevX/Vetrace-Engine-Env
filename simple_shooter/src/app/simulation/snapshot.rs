use super::*;

pub(crate) fn collect_player_snapshots(engine: &Engine, tick: u64) -> Vec<PlayerSnapshot> {
    let mut snapshots = Vec::new();
    for (actor, player, transform) in engine.query::<(&ShooterPlayer, &Transform)>() {
        let transform_snapshot = vetrace_net::ReplicatedComponentSnapshot::new(
            player.id,
            tick,
            TransformReplicator::component_name(),
            TransformSnapshot::from_transform(transform),
        );
        let velocity = actor.get_component::<Velocity>(engine).map(|v| v.linear).unwrap_or(Vec3::ZERO);
        let color_seed = actor.get_component::<PlayerGradientShader>(engine).map(|s| s.color_seed).unwrap_or(player.id);
        let weapon_id = equipped_weapon_id(engine, actor);
        snapshots.push(PlayerSnapshot {
            transform: transform_snapshot,
            velocity: v3(velocity),
            yaw: player.yaw,
            pitch: player.pitch,
            weapon_id,
            name: player.name.clone(),
            health: player.health,
            alive: player.alive,
            color_seed,
            kills: player.kills,
            deaths: player.deaths,
            last_killer_id: player.last_killer_id,
            last_killer_name: player.last_killer_name.clone(),
            last_kill_damage: player.last_kill_damage,
        });
    }
    snapshots
}

pub(crate) fn apply_snapshot(engine: &mut Engine, client: &mut ClientState, snapshot: ServerSnapshot) {
    let authoritative_ids = snapshot.states.iter().map(PlayerSnapshot::id).collect::<std::collections::BTreeSet<_>>();
    for player in snapshot.states {
        let is_local = Some(player.id()) == client.net.client_id();
        let server_position = player.position_vec3();
        let server_velocity = player.velocity_vec3();

        let actor = client.net.get_or_spawn_actor(player.id(), || {
            spawn_player(engine, player.id(), &player.name, player.color_seed, server_position, is_local)
        });

        if is_local {
            ensure_local_prediction_player(engine, actor);
        } else {
            // Snapshot-owned remote players are kinematic replicas on the client:
            // their visible Transform is driven by snapshots/interpolation, and
            // their Rapier collider follows that same Transform for local queries
            // and player-to-player collision.
            ensure_remote_snapshot_visual(engine, actor);
        }

        let previous_alive = actor.get_component::<ShooterPlayer>(engine).map(|p| p.alive).unwrap_or(player.alive);
        let alive_changed = previous_alive != player.alive;

        sync_player_gradient_material(engine, actor, player.id(), player.color_seed);

        if is_local {
            // Local client prediction owns the camera/body yaw and the immediate
            // movement feel. Do not copy every 30 Hz server yaw/velocity back
            // over it, because those snapshots are older than the current local
            // input and create visible camera/body jitter. Only hard-correct on
            // death/respawn or large drift.
            let current_position = actor.get_component::<Transform>(engine).map(|t| t.translation).unwrap_or(server_position);
            let drift_sq = current_position.distance_squared(server_position);
            let hard_correct = alive_changed
                || !player.alive
                || drift_sq > LOCAL_RECONCILE_HARD_SNAP_DISTANCE_SQ;
            if hard_correct {
                teleport_player_body(engine, actor, server_position, server_velocity);
                if alive_changed || !player.alive {
                    client.net.clear_predictions();
                    if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
                        transform.rotation = player.rotation_quat();
                    }
                } else if let Some((_, latest)) = client.net.pending_predictions().iter().last().copied() {
                    // Re-apply the most recent unacked movement intent after a
                    // hard correction so the next physics tick continues from
                    // the player's current key state instead of pausing.
                    apply_input_to_player(engine, player.id(), latest.input, latest.dt);
                }
            } else if player.alive && drift_sq > LOCAL_RECONCILE_SOFT_DISTANCE_SQ {
                // External server-authoritative motion, such as being physically
                // pushed by another player, must still reach the owning client.
                // Ignoring all small corrections made the host/other clients see
                // the physics body move while the pushed player's own camera/body
                // stayed behind. Blend the local predicted body toward the server
                // instead of snapping every snapshot.
                let corrected_position = current_position.lerp(server_position, LOCAL_RECONCILE_SOFT_ALPHA);
                let current_velocity = actor.get_component::<Velocity>(engine).map(|v| v.linear).unwrap_or(server_velocity);
                let corrected_velocity = current_velocity.lerp(server_velocity, LOCAL_RECONCILE_SOFT_ALPHA);
                teleport_player_body(engine, actor, corrected_position, corrected_velocity);
                if let Some((_, latest)) = client.net.pending_predictions().iter().last().copied() {
                    let mut replay = latest.input;
                    replay.fire = false;
                    apply_input_to_player(engine, player.id(), replay, latest.dt);
                }
            }
        } else {
            let current = actor.get_component::<Transform>(engine).map(|t| t.translation).unwrap_or(server_position);
            let should_snap = alive_changed || !player.alive || current.distance_squared(server_position) > 16.0;
            if should_snap {
                teleport_player_body(engine, actor, server_position, server_velocity);
                if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
                    transform.rotation = player.rotation_quat();
                }
                client.transform_interpolation.remove(player.id(), TransformReplicator::component_name());
            } else {
                let current_snapshot = actor.get_component::<Transform>(engine)
                    .map(TransformSnapshot::from_transform)
                    .unwrap_or_else(|| player.transform.data.clone());
                client.transform_interpolation.begin(
                    player.id(),
                    TransformReplicator::component_name(),
                    current_snapshot,
                    player.transform.data.clone(),
                    REMOTE_INTERPOLATION_SECONDS,
                );
            }
        }

        if let Some(local) = actor.get_component_mut::<ShooterPlayer>(engine) {
            local.name = player.name.clone();
            local.health = player.health;
            local.alive = player.alive;
            local.kills = player.kills;
            local.deaths = player.deaths;
            local.last_killer_id = player.last_killer_id;
            local.last_killer_name = player.last_killer_name.clone();
            local.last_kill_damage = player.last_kill_damage;
            if !is_local || alive_changed || !player.alive {
                local.yaw = player.yaw();
                local.pitch = player.pitch();
            }
        }
        let needs_weapon = actor.get_component::<EquippedWeapon>(engine)
            .map(|equipped| equipped.weapon_id != player.weapon_id)
            .unwrap_or(true);
        if needs_weapon {
            let _ = equip_weapon(engine, actor, &player.weapon_id);
        }
        set_player_visible(engine, actor, player.alive);
    }

    // Every server snapshot is a complete player set. Remove actors omitted by
    // the host so disabling bots and disconnected players disappears cleanly on
    // clients instead of leaving frozen replicas behind.
    let stale = engine.actors_with::<ShooterPlayer>().into_iter()
        .filter_map(|(actor, player)| (!authoritative_ids.contains(&player.id)).then_some((actor, player.id)))
        .collect::<Vec<_>>();
    for (actor, id) in stale {
        client.net.entity_map_mut().remove(id);
        client.transform_interpolation.remove(id, TransformReplicator::component_name());
        actor.despawn(engine);
    }
    if !authoritative_ids.is_empty() {
        despawn_orphan_outline_shells(engine);
        cleanup_orphan_player_visuals(engine);
    }

    for shot in snapshot.events {
        let from = Vec3::new(shot.from[0], shot.from[1], shot.from[2]);
        let to = Vec3::new(shot.to[0], shot.to[1], shot.to[2]);
        engine.send_event(ShotResult {
            shooter_id: shot.shooter_id,
            weapon_id: shot.weapon_id,
            muzzle: from,
            endpoint: to,
            hit_id: shot.hit_id,
        });
    }
}
