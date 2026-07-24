use super::*;

pub(super) fn apply(engine: &mut Engine, runtime: &mut ShooterRuntime, action: MainMenuAction) {
    match action {
        MainMenuAction::Play => {
            if map_count() == 0 {
                if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                    menu.status = "No maps installed. Save a map into simple_shooter/maps first.".to_string();
                }
                show_main_menu_page(engine, MainMenuPage::Maps);
                return;
            }
            let (selected_map, bot_difficulty, bot_count) = engine.get_resource::<MainMenuState>()
                .map(|menu| (menu.selected_map as u8, menu.selected_bot_difficulty, menu.selected_bot_count))
                .unwrap_or((0, BotDifficulty::Normal, runtime.config.bot_count));
            clear_all_shooter_players(engine);
            despawn_join_menu_widgets(engine);
            if let Some(join_menu) = engine.get_resource_mut::<ShooterJoinMenu>() { join_menu.active = false; }
            runtime.server = None;
            runtime.client = None;
            runtime.advertiser = None;
            runtime.config.mode = ShooterMode::Offline;
            runtime.local_id = Some(SERVER_AUTHORITY_ID);
            runtime.offline_initialized = false;
            runtime.name_ready = true;
            let mut session = ShooterSession::default();
            session.rules.map_index = normalize_map_index(selected_map);
            session.rules.bots_enabled = bot_count > 0;
            session.rules.bot_count = bot_count;
            session.rules.bot_difficulty = bot_difficulty;
            engine.insert_resource(session);
            if !activate_game_map(engine, selected_map) {
                let reason = engine.get_resource::<ShooterMapState>().and_then(|state| state.validation_error.clone())
                    .unwrap_or_else(|| "The selected map has no valid spawn points.".to_string());
                if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = reason; }
                show_main_menu_page(engine, MainMenuPage::Maps);
                return;
            }
            finish_main_menu(engine, runtime);
            spawn_kd_leaderboard(engine);
        }
        MainMenuAction::Servers => {
            if let Some(browser) = runtime.browser.as_mut() { browser.refresh(); }
            show_servers_page(engine, runtime);
        }
        MainMenuAction::RefreshServers => {
            if let Some(browser) = runtime.browser.as_mut() { browser.refresh(); }
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = "Searching the local network...".to_string(); }
            show_servers_page(engine, runtime);
        }
        MainMenuAction::HostServer => {
            let (selected_map, bot_count, max_players, bot_difficulty) = engine.get_resource::<MainMenuState>()
                .map(|menu| (menu.selected_map as u8, menu.selected_bot_count, menu.selected_max_players, menu.selected_bot_difficulty))
                .unwrap_or((0, runtime.config.bot_count, runtime.config.max_players, BotDifficulty::Normal));
            runtime.config.bot_count = bot_count;
            runtime.config.max_players = max_players.clamp(1, minimum_map_capacity());
            match runtime.host_from_menu() {
                Ok(()) => {
                    clear_all_shooter_players(engine);
                    activate_lobby_map(engine);
                    let first_mod_enabled = selected_shooter_mod(engine, 0).map(|info| info.enabled).unwrap_or(false);
                    if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
                        session.phase = MatchPhase::Lobby;
                        session.local_is_admin = true;
                        session.admin_id = SERVER_AUTHORITY_ID;
                        session.server_name = format!("{}'s server", runtime.config.player_name);
                        session.controls_open = true;
                        session.rules.mod_enabled = first_mod_enabled;
                        session.rules.map_index = normalize_map_index(selected_map);
                        session.rules.bots_enabled = bot_count > 0;
                        session.rules.bot_count = bot_count;
                        session.rules.bot_difficulty = bot_difficulty;
                        session.rules.max_players = runtime.config.max_players;
                    }
                    finish_main_menu(engine, runtime);
                    setup_lobby_ui(engine);
                }
                Err(error) => {
                    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = format!("Could not host: {error}"); }
                    show_servers_page(engine, runtime);
                }
            }
        }
        MainMenuAction::PreviousServer => {
            let count = runtime.browser.as_ref().map(|browser| browser.servers.len()).unwrap_or(0);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                if count > 0 { menu.selected_server = (menu.selected_server + count - 1) % count; }
                menu.server_summary.clear();
            }
            show_servers_page(engine, runtime);
        }
        MainMenuAction::NextServer => {
            let count = runtime.browser.as_ref().map(|browser| browser.servers.len()).unwrap_or(0);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                if count > 0 { menu.selected_server = (menu.selected_server + 1) % count; }
                menu.server_summary.clear();
            }
            show_servers_page(engine, runtime);
        }
        MainMenuAction::JoinServer => {
            let selected = engine.get_resource::<MainMenuState>().map(|menu| menu.selected_server).unwrap_or(0);
            let target = runtime.browser.as_ref().and_then(|browser| browser.servers.get(selected)).map(|server| (server.addr, server.name.clone(), server.in_lobby));
            if let Some((addr, name, in_lobby)) = target {
                if !in_lobby {
                    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = "That match has already started".to_string(); }
                    show_servers_page(engine, runtime);
                    return;
                }
                match runtime.join_from_menu(addr) {
                    Ok(()) => {
                        clear_all_shooter_players(engine);
                        activate_lobby_map(engine);
                        if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
                            session.phase = MatchPhase::Lobby;
                            session.local_is_admin = false;
                            session.server_name = name;
                            session.controls_open = false;
                        }
                        finish_main_menu(engine, runtime);
                        setup_lobby_ui(engine);
                    }
                    Err(error) => {
                        if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.status = format!("Could not join: {error}"); }
                        show_servers_page(engine, runtime);
                    }
                }
            }
        }
        _ => unreachable!("non-session main-menu action routed to session handler"),
    }
}
