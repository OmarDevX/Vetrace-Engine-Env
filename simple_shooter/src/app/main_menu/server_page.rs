use super::*;

pub(crate) fn server_page_summary(runtime: &ShooterRuntime) -> String {
    let Some(browser) = runtime.browser.as_ref() else { return "Server discovery unavailable".to_string(); };
    if browser.servers.is_empty() { return "No LAN servers found yet.\nPress REFRESH, or host one on this machine.".to_string(); }
    browser.servers.iter().enumerate().map(|(index, server)| format!("{} {}  {}/{}  {}  {}\n   {}", if index == 0 { "•" } else { " " }, server.name, server.players, server.max_players, server.map, if server.in_lobby { "LOBBY" } else { "PLAYING" }, server.addr)).collect::<Vec<_>>().join("\n")
}

pub(crate) fn show_servers_page(engine: &mut Engine, runtime: &ShooterRuntime) {
    despawn_main_menu_page(engine);
    let count = runtime.browser.as_ref().map(|browser| browser.servers.len()).unwrap_or(0);
    let selected = engine.get_resource::<MainMenuState>().map(|menu| menu.selected_server).unwrap_or(0).min(count.saturating_sub(1));
    let mut body = runtime.browser.as_ref().and_then(|browser| browser.servers.get(selected)).map(|server| format!("{}\n{}\n\nPlayers: {}/{}   Map: {}   State: {}", server.name, server.addr, server.players, server.max_players, server.map, if server.in_lobby { "Lobby (combat enabled)" } else { "Game in progress" })).unwrap_or_else(|| "No LAN servers found yet.\n\nPress REFRESH, or host a server on this machine.".to_string());
    if let Some(status) = engine.get_resource::<MainMenuState>().map(|menu| menu.status.clone()) { body.push_str(&format!("\n\n{status}")); }
    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.page = MainMenuPage::Servers; menu.selected_server = selected; menu.server_summary = server_page_summary(runtime); }
    spawn_menu_panel(engine, Vec2::new(0.0, 0.5), Vec2::new(310.0, 35.0), Vec2::new(510.0, 410.0), true);
    spawn_menu_label(engine, "SERVERS", Vec2::new(0.0, 0.5), Vec2::new(310.0, -125.0), Vec2::new(420.0, 44.0), 25.0, Vec3::ONE, true);
    spawn_menu_label(engine, &body, Vec2::new(0.0, 0.5), Vec2::new(310.0, -10.0), Vec2::new(420.0, 190.0), 15.0, Vec3::new(0.72, 0.79, 0.88), true);
    for (text, action, x, width) in [("HOST", MainMenuAction::HostServer, 130.0, 90.0), ("REFRESH", MainMenuAction::RefreshServers, 235.0, 105.0), ("‹", MainMenuAction::PreviousServer, 315.0, 42.0), ("›", MainMenuAction::NextServer, 365.0, 42.0), ("JOIN", MainMenuAction::JoinServer, 465.0, 120.0)] {
        spawn_menu_button(engine, text, action, Vec2::new(0.0, 0.5), Vec2::new(x, 150.0), Vec2::new(width, 44.0), if matches!(action, MainMenuAction::JoinServer) { menu_play_button_style() } else { menu_dark_button_style() }, if matches!(action, MainMenuAction::JoinServer) { Vec3::new(0.96, 0.70, 0.04) } else { Vec3::new(0.06, 0.09, 0.13) }, true);
    }
    spawn_menu_button(engine, "×", MainMenuAction::ClosePage, Vec2::new(0.0, 0.5), Vec2::new(520.0, -135.0), Vec2::new(42.0, 42.0), menu_dark_button_style(), Vec3::new(0.08, 0.10, 0.14), true);
}

pub(crate) fn finish_main_menu(engine: &mut Engine, runtime: &mut ShooterRuntime) {
    runtime.local_seed = engine.get_resource::<MainMenuState>().map(|menu| menu.color_roll).unwrap_or(runtime.local_seed);
    if let Some(client) = runtime.client.as_mut() {
        client.color_seed = runtime.local_seed;
        client.net.set_hello(ShooterHello { name: client.name.clone(), color_seed: client.color_seed });
    }
    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() { menu.active = false; }
    despawn_main_menu_page(engine);
    for actor in engine.actors_with::<MainMenuWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>() { actor.despawn(engine); }
    let previews = engine.actors_with::<MainMenuPreviewPlayer>().into_iter().map(|(actor, _)| actor).chain(engine.actors_with::<MainMenuPreviewStage>().into_iter().map(|(actor, _)| actor)).chain(engine.actors_with::<MainMenuPreviewOutline>().into_iter().map(|(actor, _)| actor)).collect::<Vec<_>>();
    for actor in previews { actor.despawn(engine); }
    despawn_orphan_outline_shells(engine);
    let vignette = engine.get_resource::<ShooterGameSettings>().map(|settings| settings.vignette).unwrap_or(runtime.config.post_vignette);
    sync_main_menu_post_process(engine, false, vignette);
    if runtime.config.prompt_player_name_in_ui && !runtime.name_ready { setup_join_menu_ui(engine, &runtime.config); }
    else if let Some(settings) = engine.get_resource_mut::<RenderSettings>() { settings.cursor_grab = !runtime.editor_enabled; settings.cursor_visible = runtime.editor_enabled; }
}
