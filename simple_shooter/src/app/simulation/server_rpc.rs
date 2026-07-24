use super::*;

pub(crate) fn receive_server_packets(engine: &mut Engine, server: &mut ServerState, dt: f32) {
    server.net.set_compatibility(shooter_compatibility(engine));
    let stale = server.net.timed_out_clients(CLIENT_DISCONNECT_TIMEOUT_SECONDS);
    for addr in stale { disconnect_server_client(engine, server, addr); }
    for event in server.net.poll(dt) {
        match event {
            ServerGameEvent::JoinRequested { addr, payload: ShooterHello { name, color_seed } } => {
                let accepting_players = engine.get_resource::<ShooterSession>().map(|session| session.phase.is_lobby()).unwrap_or(true);
                if !accepting_players {
                    server.net.reject(addr, "Match already started; join a server that is still in its lobby.");
                    continue;
                }
                let player_count = server.net.clients().count() as u16 + if server.transport_player_present { 1 } else { 0 };
                if !server.net.is_connected(addr) && player_count >= server.max_players {
                    server.net.reject(addr, format!("Server is full ({}/{})", player_count, server.max_players));
                    continue;
                }
                let name = sanitize_player_name(&name);
                let welcome = ShooterWelcome { mod_settings: shooter_mod_settings(engine) };
                server.net.accept_with(addr, |id| {
                    let position = spawn_position_for_active_map(engine, id);
                    let actor = spawn_player(engine, id, &name, color_seed, position, false);
                    println!("client {addr} joined as {name} ({id})");
                    (Some(actor), ShooterClientData {
                        name: name.clone(),
                        color_seed,
                        last_input: ShooterInput::default(),
                        pending_fire: None,
                    })
                }, welcome);
                let session = engine.get_resource::<ShooterSession>().cloned().unwrap_or_default();
                server.net.send_message(addr, ShooterMessage::Session {
                    phase: session.phase,
                    admin_id: session.admin_id,
                    rules: session.rules,
                });
                send_hosted_map_manifest(server, addr, session.rules.map_index);
            }
            ServerGameEvent::Input { addr, input, .. } => {
                if let Some(client) = server.net.client_mut(addr) {
                    client.game.last_input = ShooterInput::from(input);
                }
            }
            ServerGameEvent::Message { addr, client_id, message } => match message {
                ShooterMessage::AdminCommand(command) => {
                    let admin_id = engine.get_resource::<ShooterSession>().map(|session| session.admin_id);
                    if Some(client_id) == admin_id { apply_admin_command(engine, command); }
                }
                ShooterMessage::MapRequest { revision, first_missing_chunk } => {
                    send_hosted_map_chunks(server, addr, revision, first_missing_chunk);
                }
                _ => {}
            },
            ServerGameEvent::DisconnectRequested { addr } => disconnect_server_client(engine, server, addr),
        }
    }
    handle_server_rpcs(server);
}

pub(crate) fn disconnect_server_client(engine: &mut Engine, server: &mut ServerState, addr: SocketAddr) {
    if let Some(client) = server.net.remove_client(addr) {
        let departing_id = client.id;
        if let Some(actor) = client.actor() { actor.despawn(engine); }
        let replacement = choose_replacement_admin(server, departing_id);
        if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
            if session.admin_id == departing_id {
                session.admin_id = replacement.unwrap_or(SERVER_AUTHORITY_ID);
                session.local_is_admin = session.admin_id == SERVER_AUTHORITY_ID && server.transport_player_present;
                session.controls_open = session.local_is_admin;
            }
        }
        despawn_orphan_outline_shells(engine);
        despawn_orphan_name_labels(engine);
        cleanup_orphan_player_visuals(engine);
    }
}

pub(crate) fn choose_replacement_admin(server: &ServerState, departing_id: u64) -> Option<u64> {
    let mut ids = server.net.clients().filter(|client| client.id != departing_id).map(|client| client.id).collect::<Vec<_>>();
    ids.sort_unstable();
    choose_random_player_id(&ids, server.net.tick() ^ departing_id.rotate_left(17))
}

pub(crate) fn choose_random_player_id(ids: &[u64], entropy_hint: u64) -> Option<u64> {
    if ids.is_empty() { return None; }
    let entropy = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_nanos() as u64).unwrap_or(entropy_hint) ^ entropy_hint;
    Some(ids[entropy as usize % ids.len()])
}

pub(crate) fn handle_server_rpcs(server: &mut ServerState) {
    for call in server.net.drain_rpcs().collect::<Vec<_>>() {
        match call.payload {
            ShooterRpc::FireWeapon { yaw, pitch } => {
                let Some(client_id) = call.from_client_id else { continue; };
                if let Some(client) = server.net.clients_mut().find(|client| client.id == client_id) {
                    client.game.pending_fire = Some((yaw, pitch));
                }
            }
        }
    }
}

pub(crate) fn flush_post_physics_host_snapshot(engine: &Engine, runtime: &mut ShooterRuntime) {
    if !runtime.name_ready {
        return;
    }
    if !matches!(runtime.config.mode, ShooterMode::Host) {
        return;
    }
    let Some(server) = &mut runtime.server else { return; };
    if server.snapshot_due_after_physics || server.net.has_pending_events() {
        server.snapshot_due_after_physics = false;
        send_snapshot(engine, server);
    }
}

pub(crate) fn send_snapshot(engine: &Engine, server: &mut ServerState) {
    let players = collect_player_snapshots(engine, server.net.tick());
    server.net.flush_snapshot(players);
}
