use super::*;

pub(crate) fn bot_name(index: usize) -> String { format!("Bot {}", index + 1) }

pub(crate) fn bot_seed(index: usize) -> u64 { 222_u64.wrapping_add(index as u64 * 111) }

pub(crate) const HOST_BOT_ID_BASE: u64 = 10_001;

pub(crate) fn update_offline(engine: &mut Engine, runtime: &mut ShooterRuntime, dt: f32) {
    if !runtime.name_ready { return; }
    if !runtime.offline_initialized {
        spawn_player(engine, SERVER_AUTHORITY_ID, &runtime.config.player_name, runtime.local_seed, spawn_position_for_slot(engine, 0), true);
        let requested = engine.get_resource::<ShooterSession>().map(|session| if session.rules.bots_enabled { session.rules.bot_count as usize } else { 0 }).unwrap_or(runtime.config.bot_count as usize);
        let bot_slots = active_map_capacity(engine).saturating_sub(1).min(requested);
        for offset in 0..bot_slots {
            let name = bot_name(offset);
            let actor = spawn_player(engine, FIRST_REMOTE_PLAYER_ID + offset as u64, &name, bot_seed(offset), spawn_position_for_slot(engine, offset + 1), false);
            let _ = actor.insert(engine, ShooterBot);
            let _ = actor.insert(engine, ShooterBotNavigation::default());
        }
        runtime.offline_initialized = true;
    }

    let paused = engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false);
    let ids: Vec<u64> = engine.actors_with::<ShooterPlayer>().into_iter().map(|(_, player)| player.id).collect();
    for id in ids {
        let input = if Some(id) == runtime.local_id && (paused || runtime.editor_enabled || local_player_is_free_flying(engine, id)) {
            idle_input_for_player(engine, id)
        } else if Some(id) == runtime.local_id && !runtime.config.use_scripted_input {
            read_first_person_input(engine, id)
        } else if find_player_actor(engine, id).map(|actor| actor.has::<ShooterBot>(engine)).unwrap_or(false) {
            bot_combat_input(engine, id, dt)
        } else {
            scripted_input(runtime.time, id, true)
        };
        apply_input_to_player(engine, id, input, dt);
    }
    let _ = process_fire_requests(engine);
    update_respawns(engine, dt);
}

pub(crate) fn update_host(engine: &mut Engine, runtime: &mut ShooterRuntime, dt: f32) {
    if !runtime.name_ready { return; }
    if runtime.local_host_participating && engine.actors_with::<ShooterPlayer>().into_iter().all(|(_, player)| player.id != SERVER_AUTHORITY_ID) {
        let position = spawn_position_for_active_map(engine, SERVER_AUTHORITY_ID);
        spawn_player(engine, SERVER_AUTHORITY_ID, &runtime.config.player_name, runtime.local_seed, position, true);
    }

    let (bots_enabled, bot_count, max_players) = engine.get_resource::<ShooterSession>()
        .map(|session| (session.rules.bots_enabled, session.rules.bot_count, session.rules.max_players))
        .unwrap_or((false, 0, runtime.config.max_players));
    runtime.config.max_players = max_players.clamp(1, active_map_capacity(engine).max(1) as u16);
    if let Some(server) = runtime.server.as_mut() { server.max_players = runtime.config.max_players; }
    sync_host_bots(engine, bots_enabled, bot_count);

    if let Some(server) = &mut runtime.server {
        receive_server_packets(engine, server, dt);
        update_round_lifecycle(engine, dt);
        let paused = engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false);
        let host_controls = engine.get_resource::<ShooterSession>().map(|session| session.controls_open).unwrap_or(false);
        let input_blocked = engine.get_resource::<ShooterSession>().map(|session| session.phase.is_results()).unwrap_or(false);
        let host_input = if paused || host_controls || input_blocked || runtime.editor_enabled || local_player_is_free_flying(engine, SERVER_AUTHORITY_ID) {
            idle_input_for_player(engine, SERVER_AUTHORITY_ID)
        } else if runtime.config.use_scripted_input {
            scripted_input(runtime.time, SERVER_AUTHORITY_ID, true)
        } else {
            read_first_person_input(engine, SERVER_AUTHORITY_ID)
        };
        if runtime.local_host_participating {
            apply_input_to_player(engine, SERVER_AUTHORITY_ID, host_input, dt);
        }

        let client_inputs: Vec<(u64, ShooterInput)> = server.net.clients_mut().map(|client| {
            let mut input = client.game.last_input;
            if let Some((yaw, pitch)) = client.game.pending_fire.take() {
                input.yaw = yaw;
                input.pitch = pitch;
                input.fire = true;
            }
            (client.id, input)
        }).collect();
        for (id, input) in client_inputs {
            let applied = if input_blocked { idle_input_for_player(engine, id) } else { input };
            apply_input_to_player(engine, id, applied, dt);
        }
        let bot_ids = engine.actors_with::<ShooterBot>().into_iter()
            .filter_map(|(actor, _)| actor.get_component::<ShooterPlayer>(engine).map(|player| player.id))
            .collect::<Vec<_>>();
        for id in bot_ids {
            let input = if input_blocked { idle_input_for_player(engine, id) } else { bot_combat_input(engine, id, dt) };
            apply_input_to_player(engine, id, input, dt);
        }
        server.net.queue_events(process_fire_requests(engine));
        update_respawns(engine, dt);

        server.net.advance_tick();
        // Decide snapshot cadence during the simulation step, but do not send
        // yet. Physics runs after App::update, and it owns the final body
        // rotation/tilt for this frame. App::render flushes the snapshot after
        // physics has synced Transform back to the ECS world.
        let force_snapshot = server.net.has_pending_events();
        if server.net.should_send_snapshot(dt, force_snapshot) {
            server.snapshot_due_after_physics = true;
        }

        let settings = shooter_mod_settings(engine);
        server.mod_settings_resend_elapsed += dt.max(0.0);
        if server.last_mod_settings != Some(settings) || server.mod_settings_resend_elapsed >= 1.0 {
            server.last_mod_settings = Some(settings);
            server.mod_settings_resend_elapsed = 0.0;
            let fingerprint = shooter_mod_fingerprint(engine);
            let addresses = server.net.clients().map(|client| client.addr).collect::<Vec<_>>();
            for addr in addresses {
                server.net.send_message(addr, ShooterMessage::ModSettings { mod_fingerprint: fingerprint, settings });
            }
        }
        let session = engine.get_resource::<ShooterSession>().cloned().unwrap_or_default();
        let (phase_tag, standings) = match &session.phase {
            MatchPhase::Lobby => (0, Vec::new()),
            MatchPhase::Playing => (1, Vec::new()),
            MatchPhase::Results { standings, .. } => (2, standings.clone()),
        };
        let session_value = (phase_tag, session.admin_id, session.rules, standings);
        server.session_resend_elapsed += dt.max(0.0);
        if server.last_session.as_ref() != Some(&session_value) || server.session_resend_elapsed >= 1.0 {
            server.last_session = Some(session_value);
            server.session_resend_elapsed = 0.0;
            let addresses = server.net.clients().map(|client| client.addr).collect::<Vec<_>>();
            for addr in addresses {
                server.net.send_message(addr, ShooterMessage::Session {
                    phase: session.phase.clone(),
                    admin_id: session.admin_id,
                    rules: session.rules,
                });
                send_hosted_map_manifest(server, addr, session.rules.map_index);
            }
        }
    }
    if runtime.background_hosting && runtime.server.as_ref().map(|server| server.net.clients().count() == 0).unwrap_or(false) {
        shutdown_background_server(engine, runtime);
    }
}

pub(crate) fn update_round_lifecycle(engine: &mut Engine, dt: f32) {
    let phase = engine.get_resource::<ShooterSession>().map(|session| session.phase.clone()).unwrap_or(MatchPhase::Lobby);
    if phase.is_lobby() { return; }
    if phase.is_results() {
        let return_to_lobby = if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
            if let MatchPhase::Results { remaining_seconds, .. } = &mut session.phase {
                *remaining_seconds = (*remaining_seconds - dt.max(0.0)).max(0.0);
                *remaining_seconds <= 0.0
            } else { false }
        } else { false };
        if return_to_lobby {
            deploy_session_phase(engine, DeploymentTarget::Lobby, true);
            if !engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false) { setup_lobby_ui(engine); }
        }
        return;
    }
    let kill_limit = engine.get_resource::<ShooterSession>().map(|session| session.rules.kill_limit).unwrap_or(DEFAULT_KILL_LIMIT);
    let winner = engine.actors_with::<ShooterPlayer>().into_iter().any(|(_, player)| player.kills >= kill_limit);
    if !winner { return; }
    let mut standings = collect_ranked_standings(engine);
    standings.truncate(3);
    if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
        session.phase = MatchPhase::Results { remaining_seconds: ROUND_RESULTS_DURATION_SECONDS, standings };
        session.controls_open = false;
    }
    if !engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false) { setup_lobby_ui(engine); }
}

pub(crate) fn sync_host_bots(engine: &mut Engine, enabled: bool, requested_count: u16) {
    let mut existing = engine.actors_with::<ShooterBot>().into_iter().filter_map(|(actor, _)| actor.get_component::<ShooterPlayer>(engine).map(|player| (actor, player.id))).collect::<Vec<_>>();
    existing.sort_by_key(|(_, id)| *id);
    if !enabled {
        for (actor, _) in existing { actor.despawn(engine); }
        despawn_orphan_outline_shells(engine);
        return;
    }
    let occupied = engine.actors_with::<ShooterPlayer>().into_iter().filter(|(_, player)| player.id < HOST_BOT_ID_BASE).count();
    let desired = active_map_capacity(engine).saturating_sub(occupied).min(requested_count as usize);
    for (actor, _) in existing.iter().skip(desired).copied() { actor.despawn(engine); }
    for offset in existing.len()..desired {
        let id = HOST_BOT_ID_BASE + offset as u64;
        let position = spawn_position_for_slot(engine, occupied + offset);
        let name = bot_name(offset);
        let actor = spawn_player(engine, id, &name, bot_seed(offset), position, false);
        let _ = actor.insert(engine, ShooterBot);
        let _ = actor.insert(engine, ShooterBotNavigation::default());
    }
}

pub(crate) fn update_client(engine: &mut Engine, runtime: &mut ShooterRuntime, dt: f32) {
    if !runtime.name_ready { return; }
    let paused = engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false);
    let blocked = engine.get_resource::<ShooterSession>().map(|session| session.controls_open || session.phase.is_results()).unwrap_or(false);
    let input = if paused || blocked || runtime.editor_enabled || runtime.local_id.map(|id| local_player_is_free_flying(engine, id)).unwrap_or(false) {
        runtime.local_id.map(|id| idle_input_for_player(engine, id)).unwrap_or_default()
    } else if runtime.config.use_scripted_input {
        scripted_input(runtime.time, runtime.local_id.unwrap_or(0), true)
    } else if let Some(local_id) = runtime.local_id {
        read_first_person_input(engine, local_id)
    } else {
        ShooterInput::default()
    };
    let mut rejection = None;
    let mut completed_hosted_map = false;
    if let Some(client) = &mut runtime.client {
        client.net.set_compatibility(shooter_compatibility(engine));

        let fire_requested = input.fire;
        let mut movement_input = input;
        // Movement is high-rate input. Weapon activation is a named RPC so games
        // can learn the same flow used for chat, interact, reload, emotes, etc.
        movement_input.fire = false;
        let seq = client
            .net
            .send_input(NetInput::from(movement_input))
            .map(|(seq, _)| seq)
            .unwrap_or(0);

        if fire_requested && client.net.client_id().is_some() {
            client.net.rpc_named(
                "fire_weapon",
                RpcTarget::Server,
                ShooterRpc::FireWeapon { yaw: input.yaw, pitch: input.pitch },
            );
        }

        // Client-side prediction for local movement/camera. Fire/damage remains
        // server-authoritative, so predicted input suppresses local damage.
        if let Some(local_id) = client.net.client_id() {
            let predicted = movement_input;
            apply_input_to_player(engine, local_id, predicted, dt);
            client.net.push_prediction(seq, PredictedInput { input: predicted, dt: dt.clamp(0.0, 0.05) });
        }

        let received = client.net.poll(dt);
        for event in received {
            match event {
                ClientGameEvent::Joined { client_id, tick, payload: ShooterWelcome { mod_settings } } => {
                    apply_authoritative_mod_settings(engine, mod_settings);
                    if client.net.client_id() != Some(client_id) {
                        println!("joined server as id {client_id} at tick {tick}");
                    }
                    runtime.local_id = Some(client_id);
                }
                ClientGameEvent::Snapshot(snapshot) => apply_snapshot(engine, client, snapshot),
                ClientGameEvent::Message(ShooterMessage::ModSettings { mod_fingerprint: _, settings }) => {
                    apply_authoritative_mod_settings(engine, settings)
                }
                ClientGameEvent::Message(ShooterMessage::Session { phase, admin_id, rules }) => {
                    if rules.map_index as usize >= BUILTIN_MAP_COUNT && !client.hosted_map_revisions.contains_key(&rules.map_index) {
                        client.pending_hosted_session = Some(PendingHostedSession { phase, admin_id, rules });
                    } else {
                        apply_client_session(engine, runtime.local_id, phase, admin_id, rules);
                    }
                }
                ClientGameEvent::Message(ShooterMessage::MapManifest(manifest)) => {
                    if begin_client_map_transfer(client, manifest) {
                        if let Some(pending) = client.pending_hosted_session.take() {
                            apply_client_session(engine, runtime.local_id, pending.phase, pending.admin_id, pending.rules);
                        }
                    }
                }
                ClientGameEvent::Message(ShooterMessage::MapChunk { revision, chunk_index, bytes }) => {
                    match accept_client_map_chunk(client, revision, chunk_index, bytes) {
                        Ok(done) => completed_hosted_map |= done,
                        Err(err) => {
                            eprintln!("hosted map transfer failed: {err:#}");
                            client.map_transfer = None;
                        }
                    }
                }
                ClientGameEvent::Message(ShooterMessage::Shutdown { reason }) => rejection = Some(reason),
                ClientGameEvent::Message(ShooterMessage::Text { text }) => println!("server: {text}"),
                ClientGameEvent::Rejected(reason) => rejection = Some(reason),
                ClientGameEvent::Message(_) => {}
            }
        }
        update_client_map_request(client, dt);
        if completed_hosted_map {
            if let Some(pending) = client.pending_hosted_session.take() {
                apply_client_session(engine, runtime.local_id, pending.phase, pending.admin_id, pending.rules);
            }
        }
        update_client_interpolation(engine, client, dt);
        match client.net.timeout(JOIN_TIMEOUT_SECONDS, SERVER_LOSS_TIMEOUT_SECONDS) {
            Some(ClientTimeout::ServerLost) => rejection = Some("Connection to the host was lost. You were returned safely to the main menu.".to_string()),
            Some(ClientTimeout::Join) => rejection = Some("The host did not respond. The server may have closed.".to_string()),
            None => {}
        }
    }
    if let Some(reason) = rejection {
        leave_game_to_main_menu(engine, runtime);
        if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = reason; }
    }
}
