use super::*;

pub(crate) fn update_server_directory(engine: &Engine, runtime: &mut ShooterRuntime) {
    if let Some(browser) = runtime.browser.as_mut() { browser.update(); }
    let Some(advertiser) = runtime.advertiser.as_ref() else { return; };
    let players = runtime.server.as_ref().map(|server| server.net.clients().count() as u16 + if runtime.local_host_participating { 1 } else { 0 }).unwrap_or(1);
    let session = engine.get_resource::<ShooterSession>().cloned().unwrap_or_default();
    let map = map_name(session.rules.map_index);
    advertiser.update(players, runtime.config.max_players, &map, session.phase.is_lobby());
}

pub(crate) fn setup_lobby_ui(engine: &mut Engine) {
    despawn_lobby_widgets(engine, false);
    let session = engine.get_resource::<ShooterSession>().cloned().unwrap_or_default();
    let status_text = lobby_status_text(engine, &session);
    engine.spawn_actor("Lobby status")
        .with(LobbyWidget { action: None, host_control: false })
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.0), offset_px: Vec2::new(0.0, 30.0), size_px: Vec2::new(640.0, 42.0), z_order: 360 })
        .with(vetrace_ui::UIPanel { size: Vec2::new(640.0, 42.0), background: Vec3::new(0.02, 0.03, 0.05), alpha: 0.82, anchor: Anchor::Center })
        .with(vetrace_ui::UIVisualStyle::rounded(10.0).with_border(1.0, Vec3::new(0.2, 0.5, 0.8), 0.7))
        .build();
    engine.spawn_actor("Lobby status label")
        .with(LobbyWidget { action: None, host_control: false })
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.0), offset_px: Vec2::new(0.0, 30.0), size_px: Vec2::new(620.0, 36.0), z_order: 361 })
        .with(UILabel { text: status_text, font_size: 15.0, color: Vec3::ONE, anchor: Anchor::Center, align: TextAlign::Center })
        .build();
    spawn_kd_leaderboard(engine);
    if session.phase.is_results() { spawn_round_results(engine, &session); }
    if session.local_is_admin && session.controls_open { spawn_host_lobby_controls(engine, &session); }
}

pub(crate) fn spawn_kd_leaderboard(engine: &mut Engine) {
    if !engine.actors_with::<LeaderboardWidget>().is_empty() { return; }
    engine.spawn_actor("K/D leaderboard")
        .with(LeaderboardWidget)
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(1.0, 0.0), offset_px: Vec2::new(-170.0, 105.0), size_px: Vec2::new(320.0, 170.0), z_order: 350 })
        .with(UILabel { text: "K/D LEADERBOARD".to_string(), font_size: 15.0, color: Vec3::new(0.92, 0.95, 1.0), anchor: Anchor::Center, align: TextAlign::Center })
        .build();
}

pub(crate) fn lobby_status_text(engine: &Engine, session: &ShooterSession) -> String {
    if let Some(error) = engine.get_resource::<ShooterMapState>().and_then(|state| state.validation_error.as_ref()) {
        return format!("MAP NOT READY • {error}");
    }
    if session.phase.is_lobby() {
        format!("LOBBY • Combat enabled • {} • Map: {} • First to {}{}", session.server_name, map_name(session.rules.map_index), session.rules.kill_limit, if session.local_is_admin { " • F1 admin controls" } else { " • Waiting for admin" })
    } else if session.phase.is_results() {
        "ROUND OVER • Results".to_string()
    } else {
        format!("MATCH STARTED • {} • First to {}{}", map_name(session.rules.map_index), session.rules.kill_limit, if session.local_is_admin { " • F1 admin controls" } else { "" })
    }
}

pub(crate) fn spawn_host_lobby_controls(engine: &mut Engine, session: &ShooterSession) {
    if !session.phase.is_lobby() {
        lobby_panel(engine, Vec2::new(-425.0, 35.0), Vec2::new(390.0, 190.0));
        lobby_label(engine, "GAME ADMIN", Vec2::new(-425.0, -10.0), Vec2::new(350.0, 36.0), 21.0);
        lobby_button(engine, "STOP GAME / RETURN TO LOBBY", LobbyAction::StopGame, Vec2::new(-425.0, 60.0), Vec2::new(310.0, 48.0));
        return;
    }
    lobby_panel(engine, Vec2::new(-425.0, 55.0), Vec2::new(390.0, 690.0));
    lobby_label(engine, "HOST LOBBY SETTINGS", Vec2::new(-425.0, -265.0), Vec2::new(350.0, 36.0), 21.0);
    lobby_label(engine, &format!("Map: {}\nLua mod: {} ({})\nBots: {} • Count: {} ({})\nMax human players: {}\nMove speed: {:.2}x\nGravity: {:.2}x\nJump: {:.2}x\nKills to win: {}\n\nLobby combat stays active.", map_name(session.rules.map_index), lobby_mod_name(engine, session.rules.mod_index), if session.rules.mod_enabled { "enabled" } else { "disabled" }, if session.rules.bots_enabled { "ON" } else { "OFF" }, session.rules.bot_count, session.rules.bot_difficulty.name(), session.rules.max_players, session.rules.move_speed_multiplier, session.rules.gravity_scale, session.rules.jump_multiplier, session.rules.kill_limit), Vec2::new(-425.0, -125.0), Vec2::new(340.0, 245.0), 15.0);
    lobby_button(engine, "MAP ‹", LobbyAction::PreviousMap, Vec2::new(-510.0, 35.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "MAP ›", LobbyAction::NextMap, Vec2::new(-365.0, 35.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "MOD ‹", LobbyAction::PreviousMod, Vec2::new(-510.0, 82.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "MOD ›", LobbyAction::NextMod, Vec2::new(-365.0, 82.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "ENABLE / DISABLE MOD", LobbyAction::ToggleMod, Vec2::new(-425.0, 128.0), Vec2::new(275.0, 38.0));
    lobby_button(engine, "BOTS", LobbyAction::ToggleBots, Vec2::new(-525.0, 174.0), Vec2::new(75.0, 38.0));
    lobby_button(engine, "−", LobbyAction::BotCountDown, Vec2::new(-445.0, 174.0), Vec2::new(45.0, 38.0));
    lobby_button(engine, "+", LobbyAction::BotCountUp, Vec2::new(-395.0, 174.0), Vec2::new(45.0, 38.0));
    lobby_button(engine, "DIFF ‹", LobbyAction::DifficultyDown, Vec2::new(-340.0, 174.0), Vec2::new(75.0, 38.0));
    lobby_button(engine, "DIFF ›", LobbyAction::DifficultyUp, Vec2::new(-260.0, 174.0), Vec2::new(75.0, 38.0));
    lobby_button(engine, "PLAYERS −", LobbyAction::MaxPlayersDown, Vec2::new(-510.0, 221.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "PLAYERS +", LobbyAction::MaxPlayersUp, Vec2::new(-365.0, 221.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "SPEED −", LobbyAction::SpeedDown, Vec2::new(-510.0, 268.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "SPEED +", LobbyAction::SpeedUp, Vec2::new(-365.0, 268.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "GRAVITY −", LobbyAction::GravityDown, Vec2::new(-510.0, 315.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "GRAVITY +", LobbyAction::GravityUp, Vec2::new(-365.0, 315.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "JUMP −", LobbyAction::JumpDown, Vec2::new(-510.0, 362.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "JUMP +", LobbyAction::JumpUp, Vec2::new(-365.0, 362.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "KILLS −", LobbyAction::KillLimitDown, Vec2::new(-510.0, 409.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "KILLS +", LobbyAction::KillLimitUp, Vec2::new(-365.0, 409.0), Vec2::new(130.0, 38.0));
    lobby_button(engine, "START GAME", LobbyAction::StartGame, Vec2::new(-425.0, 468.0), Vec2::new(275.0, 48.0));
}

pub(crate) fn spawn_round_results(engine: &mut Engine, session: &ShooterSession) {
    let mut text = String::from("ROUND OVER\n\nTOP 3 PLAYERS\n");
    let (standings, remaining) = match &session.phase { MatchPhase::Results { standings, remaining_seconds } => (standings.as_slice(), *remaining_seconds), _ => (&[][..], 0.0) };
    for (rank, row) in standings.iter().enumerate() {
        text.push_str(&format!("{}. {}  —  {} K / {} D\n", rank + 1, row.name, row.kills, row.deaths));
    }
    text.push_str(&format!("\nReturning to the lobby in {:.0}...", remaining.ceil()));
    engine.spawn_actor("Round results panel").with(RoundResultsWidget).with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::splat(0.5), offset_px: Vec2::ZERO, size_px: Vec2::new(560.0, 360.0), z_order: 700 })
        .with(vetrace_ui::UIPanel { size: Vec2::new(560.0, 360.0), background: Vec3::new(0.015, 0.025, 0.045), alpha: 0.96, anchor: Anchor::Center })
        .with(vetrace_ui::UIVisualStyle::rounded(18.0).with_border(2.0, Vec3::new(0.3, 0.65, 1.0), 0.9)).build();
    engine.spawn_actor("Round results text").with(RoundResultsWidget).with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::splat(0.5), offset_px: Vec2::ZERO, size_px: Vec2::new(510.0, 320.0), z_order: 701 })
        .with(UILabel { text, font_size: 22.0, color: Vec3::ONE, anchor: Anchor::Center, align: TextAlign::Center }).build();
}

pub(crate) fn lobby_panel(engine: &mut Engine, offset: Vec2, size: Vec2) {
    engine.spawn_actor("Host lobby panel").with(LobbyWidget { action: None, host_control: true }).with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.5), offset_px: offset, size_px: size, z_order: 370 })
        .with(vetrace_ui::UIPanel { size, background: Vec3::new(0.025, 0.035, 0.052), alpha: 0.94, anchor: Anchor::Center })
        .with(vetrace_ui::UIVisualStyle::rounded(16.0).with_border(1.0, Vec3::new(0.18, 0.44, 0.7), 0.8)).build();
}

pub(crate) fn lobby_label(engine: &mut Engine, text: &str, offset: Vec2, size: Vec2, font_size: f32) {
    engine.spawn_actor("Host lobby label").with(LobbyWidget { action: None, host_control: true }).with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.5), offset_px: offset, size_px: size, z_order: 371 })
        .with(UILabel { text: text.to_string(), font_size, color: Vec3::new(0.86, 0.91, 0.98), anchor: Anchor::Center, align: TextAlign::Center }).build();
}

pub(crate) fn lobby_button(engine: &mut Engine, text: &str, action: LobbyAction, offset: Vec2, size: Vec2) {
    engine.spawn_actor(format!("Host lobby button: {text}")).with(LobbyWidget { action: Some(action), host_control: true }).with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.5, 0.5), offset_px: offset, size_px: size, z_order: 373 })
        .with(vetrace_ui::UIButton { text: text.to_string(), size, ..vetrace_ui::UIButton::default() })
        .with(menu_dark_button_style()).with(Material { base_color: Vec3::new(0.06, 0.12, 0.19), alpha: 0.97, ..Material::default() }).build();
}

pub(crate) fn update_lobby_ui(engine: &mut Engine, runtime: &mut ShooterRuntime) {
    let exists = engine.get_resource::<ShooterSession>().map(|session| !session.server_name.is_empty()).unwrap_or(false);
    if !exists { return; }
    let f1 = engine.get_resource::<InputState>().map(|input| input.was_key_pressed("F1")).unwrap_or(false);
    if f1 {
        if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
            if session.local_is_admin && !session.phase.is_results() { session.controls_open = !session.controls_open; }
        }
        setup_lobby_ui(engine);
    }
    let session = engine.get_resource::<ShooterSession>().cloned().unwrap_or_default();
    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        if session.controls_open { settings.cursor_grab = false; settings.cursor_visible = true; }
    }
    if !(session.local_is_admin && session.controls_open) { sync_lobby_labels(engine, &session); return; }
    let action = lobby_pointer_action(engine);
    if let Some(action) = action {
        apply_lobby_action(engine, runtime, action);
        setup_lobby_ui(engine);
    }
}

pub(crate) fn lobby_pointer_action(engine: &mut Engine) -> Option<LobbyAction> {
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let viewport = Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32);
    let point = Vec2::new(input.mouse_position().0, input.mouse_position().1);
    let mut hit = None;
    let widgets = engine.actors_with::<LobbyWidget>().into_iter().filter_map(|(actor, widget)| Some((actor, widget.action?, actor.get_component::<ScreenSpaceRect>(engine)?.clone()))).collect::<Vec<_>>();
    for (actor, action, rect) in widgets {
        let interaction = vetrace_ui::pointer_interaction(viewport, rect.anchor, rect.offset_px, rect.size_px, point, input.is_mouse_button_down("Left"), input.was_mouse_button_released("Left"));
        if let Some(button) = actor.get_component_mut::<vetrace_ui::UIButton>(engine) { button.hovered = interaction.hovered; button.pressed = interaction.pressed; }
        if interaction.clicked && hit.is_none() { hit = Some(action); }
    }
    hit
}

pub(crate) fn lobby_action_command(action: LobbyAction) -> ShooterAdminCommand {
    match action {
        LobbyAction::StartGame => ShooterAdminCommand::StartGame,
        LobbyAction::StopGame => ShooterAdminCommand::StopGame,
        LobbyAction::PreviousMap => ShooterAdminCommand::PreviousMap,
        LobbyAction::NextMap => ShooterAdminCommand::NextMap,
        LobbyAction::PreviousMod => ShooterAdminCommand::PreviousMod,
        LobbyAction::NextMod => ShooterAdminCommand::NextMod,
        LobbyAction::ToggleMod => ShooterAdminCommand::ToggleMod,
        LobbyAction::ToggleBots => ShooterAdminCommand::ToggleBots,
        LobbyAction::BotCountDown => ShooterAdminCommand::BotCountDown,
        LobbyAction::BotCountUp => ShooterAdminCommand::BotCountUp,
        LobbyAction::MaxPlayersDown => ShooterAdminCommand::MaxPlayersDown,
        LobbyAction::MaxPlayersUp => ShooterAdminCommand::MaxPlayersUp,
        LobbyAction::DifficultyDown => ShooterAdminCommand::DifficultyDown,
        LobbyAction::DifficultyUp => ShooterAdminCommand::DifficultyUp,
        LobbyAction::SpeedDown => ShooterAdminCommand::SpeedDown,
        LobbyAction::SpeedUp => ShooterAdminCommand::SpeedUp,
        LobbyAction::GravityDown => ShooterAdminCommand::GravityDown,
        LobbyAction::GravityUp => ShooterAdminCommand::GravityUp,
        LobbyAction::JumpDown => ShooterAdminCommand::JumpDown,
        LobbyAction::JumpUp => ShooterAdminCommand::JumpUp,
        LobbyAction::KillLimitDown => ShooterAdminCommand::KillLimitDown,
        LobbyAction::KillLimitUp => ShooterAdminCommand::KillLimitUp,
    }
}

pub(crate) fn apply_lobby_action(engine: &mut Engine, runtime: &mut ShooterRuntime, action: LobbyAction) {
    let command = lobby_action_command(action);
    if matches!(runtime.config.mode, ShooterMode::Join) {
        if let Some(client) = runtime.client.as_mut() {
            client.net.send_message(ShooterMessage::AdminCommand(command));
        }
        return;
    }
    apply_admin_command(engine, command);
}

pub(crate) fn apply_admin_command(engine: &mut Engine, command: ShooterAdminCommand) {
    if matches!(command, ShooterAdminCommand::PreviousMod | ShooterAdminCommand::NextMod) {
        let count = shooter_mod_count(engine);
        let current = engine.get_resource::<ShooterSession>().map(|session| session.rules.mod_index as usize).unwrap_or(0);
        let selected = if count == 0 { 0 } else if command == ShooterAdminCommand::PreviousMod { (current + count - 1) % count } else { (current + 1) % count };
        let enabled = selected_shooter_mod(engine, selected).map(|info| info.enabled).unwrap_or(false);
        if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
            session.rules.mod_index = selected.min(u8::MAX as usize) as u8;
            session.rules.mod_enabled = enabled;
        }
        return;
    }
    if command == ShooterAdminCommand::ToggleMod {
        let selected = engine.get_resource::<ShooterSession>().map(|session| session.rules.mod_index as usize).unwrap_or(0);
        let _status = toggle_selected_shooter_mod(engine, selected);
        let enabled = selected_shooter_mod(engine, selected).map(|info| info.enabled).unwrap_or(false);
        if let Some(session) = engine.get_resource_mut::<ShooterSession>() { session.rules.mod_enabled = enabled; }
        return;
    }
    let mut deployment = None;
    let maps = map_count();
    if matches!(command, ShooterAdminCommand::PreviousMap | ShooterAdminCommand::NextMap) {
        set_map_validation_error(engine, String::new());
    }
    {
        let Some(session) = engine.get_resource_mut::<ShooterSession>() else { return; };
        match command {
            ShooterAdminCommand::StartGame => {
                if maps > 0 { deployment = Some(DeploymentTarget::Game(session.rules.map_index)); }
            }
            ShooterAdminCommand::StopGame => deployment = Some(DeploymentTarget::Lobby),
            ShooterAdminCommand::PreviousMap => {
                if maps > 0 { session.rules.map_index = ((session.rules.map_index as usize + maps - 1) % maps) as u8; }
            }
            ShooterAdminCommand::NextMap => {
                if maps > 0 { session.rules.map_index = ((session.rules.map_index as usize + 1) % maps) as u8; }
            }
            ShooterAdminCommand::PreviousMod | ShooterAdminCommand::NextMod | ShooterAdminCommand::ToggleMod => {}
            ShooterAdminCommand::ToggleBots => session.rules.bots_enabled = !session.rules.bots_enabled,
            ShooterAdminCommand::BotCountDown => session.rules.bot_count = session.rules.bot_count.saturating_sub(1),
            ShooterAdminCommand::BotCountUp => session.rules.bot_count = session.rules.bot_count.saturating_add(1),
            ShooterAdminCommand::MaxPlayersDown => session.rules.max_players = session.rules.max_players.saturating_sub(1).max(1),
            ShooterAdminCommand::MaxPlayersUp => session.rules.max_players = session.rules.max_players.saturating_add(1),
            ShooterAdminCommand::DifficultyDown => session.rules.bot_difficulty = session.rules.bot_difficulty.previous(),
            ShooterAdminCommand::DifficultyUp => session.rules.bot_difficulty = session.rules.bot_difficulty.next(),
            ShooterAdminCommand::SpeedDown => session.rules.move_speed_multiplier -= RULE_MULTIPLIER_STEP,
            ShooterAdminCommand::SpeedUp => session.rules.move_speed_multiplier += RULE_MULTIPLIER_STEP,
            ShooterAdminCommand::GravityDown => session.rules.gravity_scale -= RULE_MULTIPLIER_STEP,
            ShooterAdminCommand::GravityUp => session.rules.gravity_scale += RULE_MULTIPLIER_STEP,
            ShooterAdminCommand::JumpDown => session.rules.jump_multiplier -= RULE_MULTIPLIER_STEP,
            ShooterAdminCommand::JumpUp => session.rules.jump_multiplier += RULE_MULTIPLIER_STEP,
            ShooterAdminCommand::KillLimitDown => session.rules.kill_limit = session.rules.kill_limit.saturating_sub(KILL_LIMIT_STEP).max(MIN_KILL_LIMIT),
            ShooterAdminCommand::KillLimitUp => session.rules.kill_limit = session.rules.kill_limit.saturating_add(KILL_LIMIT_STEP).min(MAX_KILL_LIMIT),
        }
        session.rules = session.rules.normalized();
        session.rules.map_index = normalize_map_index(session.rules.map_index);
    }
    if let Some(target) = deployment {
        deploy_session_phase(engine, target, true);
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DeploymentTarget { Lobby, Game(u8) }

pub(crate) fn deploy_session_phase(engine: &mut Engine, target: DeploymentTarget, reset_scores: bool) -> bool {
    if let DeploymentTarget::Game(index) = target {
        if !activate_game_map(engine, index) { return false; }
    }
    if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
        match target {
            DeploymentTarget::Lobby => {
                session.phase = MatchPhase::Lobby;
                session.controls_open = session.local_is_admin;
            }
            DeploymentTarget::Game(_) => {
                session.phase = MatchPhase::Playing;
                session.controls_open = false;
            }
        }
    }
    if reset_scores { reset_player_scores(engine); }
    match target {
        DeploymentTarget::Lobby => activate_lobby_map(engine),
        DeploymentTarget::Game(_) => {}
    }
    deploy_players_for_map(engine);
    true
}

pub(crate) fn reset_player_scores(engine: &mut Engine) {
    let actors = engine.actors_with::<ShooterPlayer>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors {
        if let Some(player) = actor.get_component_mut::<ShooterPlayer>(engine) {
            player.kills = 0;
            player.deaths = 0;
            player.health = 100;
            player.alive = true;
            player.respawn_timer = 0.0;
            player.last_killer_id = None;
            player.last_killer_name.clear();
            player.last_kill_damage = 0;
            player.life_damage_by_attacker.clear();
        }
    }
}

pub(crate) fn lobby_mod_name(engine: &Engine, index: u8) -> String {
    selected_shooter_mod(engine, index as usize).map(|info| info.manifest.name).unwrap_or_else(|| "No mods found".to_string())
}

pub(crate) fn deploy_players_for_map(engine: &mut Engine) {
    let players = engine.actors_with::<ShooterPlayer>().into_iter().map(|(actor, player)| (actor, player.id)).collect::<Vec<_>>();
    for (actor, id) in players {
        let position = spawn_position_for_active_map(engine, id);
        teleport_player_body(engine, actor, position, Vec3::ZERO);
    }
}

pub(crate) fn sync_lobby_labels(engine: &mut Engine, session: &ShooterSession) {
    let labels = engine.actors_with::<LobbyWidget>().into_iter().filter_map(|(actor, widget)| (!widget.host_control).then_some(actor)).collect::<Vec<_>>();
    for actor in labels {
        let text = lobby_status_text(engine, session);
        if let Some(label) = actor.get_component_mut::<UILabel>(engine) { label.text = text; }
    }
}

pub(crate) fn update_kills_leaderboard(engine: &mut Engine) {
    let mut rows = engine.actors_with::<ShooterPlayer>().into_iter().map(|(_, player)| (player.name.clone(), player.kills, player.deaths)).collect::<Vec<_>>();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));
    let mut text = String::from("K/D LEADERBOARD\n");
    for (rank, (name, kills, deaths)) in rows.into_iter().take(8).enumerate() {
        let ratio = kills as f32 / deaths.max(1) as f32;
        text.push_str(&format!("{}. {}   {} K / {} D   {:.2} KD\n", rank + 1, name, kills, deaths, ratio));
    }
    let actors = engine.actors_with::<LeaderboardWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors { if let Some(label) = actor.get_component_mut::<UILabel>(engine) { label.text = text.clone(); } }
}

pub(crate) fn collect_ranked_standings(engine: &Engine) -> Vec<ScoreStanding> {
    let mut standings = engine.actors_with::<ShooterPlayer>().into_iter().map(|(_, player)| ScoreStanding {
        name: player.name.clone(), kills: player.kills, deaths: player.deaths,
    }).collect::<Vec<_>>();
    rank_standings(&mut standings);
    standings
}

pub(crate) fn rank_standings(standings: &mut [ScoreStanding]) {
    standings.sort_by(|a, b| b.kills.cmp(&a.kills).then(a.deaths.cmp(&b.deaths)).then(a.name.cmp(&b.name)));
}

#[cfg(test)]
mod standings_tests {
    use super::*;

    #[test]
    fn standings_use_kills_then_deaths_then_name() {
        let mut rows = vec![
            ScoreStanding { name: "Zed".into(), kills: 7, deaths: 3 },
            ScoreStanding { name: "Amy".into(), kills: 7, deaths: 2 },
            ScoreStanding { name: "Bob".into(), kills: 7, deaths: 2 },
        ];
        rank_standings(&mut rows);
        assert_eq!(rows.into_iter().map(|row| row.name).collect::<Vec<_>>(), vec!["Amy".to_string(), "Bob".to_string(), "Zed".to_string()]);
    }
}

pub(crate) fn despawn_lobby_widgets(engine: &mut Engine, host_only: bool) {
    let actors = engine.actors_with::<LobbyWidget>().into_iter().filter_map(|(actor, widget)| (!host_only || widget.host_control).then_some(actor)).collect::<Vec<_>>();
    for actor in actors { actor.despawn(engine); }
    if !host_only {
        let boards = engine.actors_with::<LeaderboardWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
        for actor in boards { actor.despawn(engine); }
        let results = engine.actors_with::<RoundResultsWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
        for actor in results { actor.despawn(engine); }
    }
}
