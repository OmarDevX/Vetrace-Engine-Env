use super::*;

pub(crate) fn update_pause_menu(engine: &mut Engine, runtime: &mut ShooterRuntime) -> bool {
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    if input.quit_requested() {
        send_client_leave(runtime);
        engine.stop();
        return true;
    }

    if !runtime.editor_enabled && input.was_key_pressed("Escape") {
        let active = !engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false);
        set_pause_menu_active(engine, active);
    }

    let active = engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false);
    if !active { return false; }

    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = false;
        settings.cursor_visible = true;
    }
    if let Some(action) = update_pause_menu_pointer(engine) {
        match action {
            PauseMenuAction::Resume => set_pause_menu_active(engine, false),
            PauseMenuAction::Settings => {
                if let Some(state) = engine.get_resource_mut::<PauseMenuState>() { state.page = PauseMenuPage::Settings; }
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::Back => {
                if let Some(state) = engine.get_resource_mut::<PauseMenuState>() { state.page = PauseMenuPage::Home; }
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::LeaveToMainMenu => leave_game_to_main_menu(engine, runtime),
            PauseMenuAction::CycleGraphics => {
                update_game_settings(engine, |settings| settings.graphics_profile = match settings.graphics_profile {
                    ShooterGraphicsProfile::LowSpec => ShooterGraphicsProfile::Balanced,
                    ShooterGraphicsProfile::Balanced => ShooterGraphicsProfile::HighQuality,
                    ShooterGraphicsProfile::HighQuality => ShooterGraphicsProfile::LowSpec,
                });
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::ToggleVignette => {
                update_game_settings(engine, |settings| settings.vignette = !settings.vignette);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::ToggleVolumetricFog => {
                update_game_settings(engine, |settings| settings.volumetric_fog = !settings.volumetric_fog);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::ToggleVsync => {
                update_game_settings(engine, |settings| settings.vsync = !settings.vsync);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::SensitivityDown => {
                update_game_settings(engine, |settings| settings.mouse_sensitivity -= 0.0004);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::SensitivityUp => {
                update_game_settings(engine, |settings| settings.mouse_sensitivity += 0.0004);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::VolumeDown => {
                update_game_settings(engine, |settings| settings.master_volume -= 0.1);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::VolumeUp => {
                update_game_settings(engine, |settings| settings.master_volume += 0.1);
                rebuild_pause_menu(engine);
            }
            PauseMenuAction::Quit => {
                send_client_leave(runtime);
                engine.stop();
            }
        }
    }
    engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false)
}

pub(crate) fn set_pause_menu_active(engine: &mut Engine, active: bool) {
    if let Some(state) = engine.get_resource_mut::<PauseMenuState>() {
        state.active = active;
        if active { state.page = PauseMenuPage::Home; }
    }
    despawn_pause_menu(engine);
    if active {
        rebuild_pause_menu(engine);
    } else if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = true;
        settings.cursor_visible = false;
    }
}

pub(crate) fn rebuild_pause_menu(engine: &mut Engine) {
    despawn_pause_menu(engine);
    let settings = engine.get_resource::<ShooterGameSettings>().cloned().unwrap_or_default();
    let page = engine.get_resource::<PauseMenuState>().map(|state| state.page).unwrap_or_default();
    spawn_pause_panel(engine);
    match page {
        PauseMenuPage::Home => {
            spawn_pause_label(engine, "PAUSED", Vec2::new(0.5, 0.5), Vec2::new(0.0, -142.0), Vec2::new(310.0, 48.0), 30.0, Vec3::ONE);
            spawn_pause_button(engine, "RESUME", PauseMenuAction::Resume, Vec2::new(0.0, -65.0), menu_play_button_style(), Vec3::new(0.96, 0.70, 0.04));
            spawn_pause_button(engine, "SETTINGS", PauseMenuAction::Settings, Vec2::new(0.0, -5.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_button(engine, "LEAVE TO MAIN MENU", PauseMenuAction::LeaveToMainMenu, Vec2::new(0.0, 55.0), menu_dark_button_style(), Vec3::new(0.08, 0.11, 0.16));
            spawn_pause_button(engine, "QUIT DESKTOP", PauseMenuAction::Quit, Vec2::new(0.0, 115.0), menu_dark_button_style(), Vec3::new(0.16, 0.055, 0.06));
        }
        PauseMenuPage::Settings => {
            spawn_pause_label(engine, "SETTINGS", Vec2::new(0.5, 0.5), Vec2::new(0.0, -166.0), Vec2::new(310.0, 42.0), 27.0, Vec3::ONE);
            spawn_pause_label(engine, &format!("{:?}  •  VSYNC {}  •  FOG {}  •  VIGNETTE {}\nSENS {:.1}x  •  VOLUME {}%", settings.graphics_profile, if settings.vsync { "ON" } else { "OFF" }, if settings.volumetric_fog { "ON" } else { "OFF" }, if settings.vignette { "ON" } else { "OFF" }, settings.mouse_sensitivity / FPS_MOUSE_SENSITIVITY, (settings.master_volume * 100.0).round() as u32), Vec2::splat(0.5), Vec2::new(0.0, -120.0), Vec2::new(390.0, 48.0), 13.0, Vec3::new(0.65, 0.74, 0.84));
            spawn_pause_button(engine, "QUALITY", PauseMenuAction::CycleGraphics, Vec2::new(0.0, -62.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_button(engine, "VSYNC", PauseMenuAction::ToggleVsync, Vec2::new(0.0, -8.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_sized_button(engine, "FOG", PauseMenuAction::ToggleVolumetricFog, Vec2::new(-77.0, 46.0), Vec2::new(145.0, 48.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_sized_button(engine, "VIGNETTE", PauseMenuAction::ToggleVignette, Vec2::new(77.0, 46.0), Vec2::new(145.0, 48.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_sized_button(engine, "SENS −", PauseMenuAction::SensitivityDown, Vec2::new(-77.0, 100.0), Vec2::new(145.0, 48.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_sized_button(engine, "SENS +", PauseMenuAction::SensitivityUp, Vec2::new(77.0, 100.0), Vec2::new(145.0, 48.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_sized_button(engine, "VOL −", PauseMenuAction::VolumeDown, Vec2::new(-77.0, 154.0), Vec2::new(145.0, 48.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_sized_button(engine, "VOL +", PauseMenuAction::VolumeUp, Vec2::new(77.0, 154.0), Vec2::new(145.0, 48.0), menu_dark_button_style(), Vec3::new(0.07, 0.10, 0.14));
            spawn_pause_button(engine, "BACK", PauseMenuAction::Back, Vec2::new(0.0, 208.0), menu_play_button_style(), Vec3::new(0.25, 0.48, 0.82));
        }
    }
    if page == PauseMenuPage::Home {
        spawn_pause_label(engine, "ESC  RESUME", Vec2::new(0.5, 0.5), Vec2::new(0.0, 178.0), Vec2::new(240.0, 24.0), 12.0, Vec3::new(0.48, 0.56, 0.65));
    }
}

pub(crate) fn leave_game_to_main_menu(engine: &mut Engine, runtime: &mut ShooterRuntime) {
    set_pause_menu_active(engine, false);
    if matches!(runtime.config.mode, ShooterMode::Host) {
        let client_ids = runtime.server.as_ref().map(|server| {
            let mut ids = server.net.clients().map(|client| client.id).collect::<Vec<_>>();
            ids.sort_unstable();
            ids
        }).unwrap_or_default();
        if !client_ids.is_empty() {
            let next_admin = choose_random_player_id(&client_ids, runtime.time.to_bits() as u64).expect("non-empty client list");
            if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
                session.admin_id = next_admin;
                session.local_is_admin = false;
                session.controls_open = false;
            }
            let host_actor = engine.actors_with::<ShooterPlayer>().into_iter().find_map(|(actor, player)| (player.id == SERVER_AUTHORITY_ID).then_some(actor));
            if let Some(actor) = host_actor { actor.despawn(engine); }
            despawn_orphan_outline_shells(engine);
            despawn_orphan_name_labels(engine);
            runtime.background_hosting = true;
            runtime.local_host_participating = false;
            if let Some(server) = runtime.server.as_mut() { server.transport_player_present = false; }
            despawn_lobby_widgets(engine, false);
            clear_transient_gameplay_visuals(engine);
            setup_main_menu(engine, runtime);
            if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
                menu.status = format!("Server still running. Player {next_admin} is now the game admin.");
            }
            hide_background_gameplay(engine);
            return;
        }
    }
    send_client_leave(runtime);
    runtime.server = None;
    runtime.client = None;
    runtime.advertiser = None;
    runtime.config.mode = ShooterMode::Offline;
    runtime.local_id = Some(SERVER_AUTHORITY_ID);
    runtime.offline_initialized = false;
    runtime.background_hosting = false;
    runtime.local_host_participating = true;
    clear_all_shooter_players(engine);
    clear_transient_gameplay_visuals(engine);
    despawn_join_menu_widgets(engine);
    if let Some(join_menu) = engine.get_resource_mut::<ShooterJoinMenu>() { join_menu.active = false; }
    despawn_lobby_widgets(engine, false);
    clear_active_map(engine);
    engine.insert_resource(ShooterSession::default());
    setup_main_menu(engine, runtime);
}

pub(crate) fn send_client_leave(runtime: &mut ShooterRuntime) {
    if let Some(client) = runtime.client.as_mut() {
        client.net.leave();
        client.net.leave();
    }
}

pub(crate) fn hide_background_gameplay(engine: &mut Engine) {
    let gameplay = engine.actors_with::<ShooterPlayer>().into_iter().map(|(actor, _)| actor)
        .chain(engine.actors_with::<ShooterMapGeometry>().into_iter().map(|(actor, _)| actor))
        .chain(engine.actors_with::<ShooterOutlineShell>().into_iter().map(|(actor, _)| actor))
        .collect::<Vec<_>>();
    for actor in gameplay {
        if let Some(renderable) = actor.get_component_mut::<Renderable>(engine) { renderable.visible = false; }
    }
    set_all_player_weapons_visible(engine, false);
    let labels = engine.actors_with::<PlayerNameLabel>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in labels {
        if let Some(world) = actor.get_component_mut::<UIWorldSpace>(engine) { world.visible = false; }
    }
}

pub(crate) fn shutdown_background_server(engine: &mut Engine, runtime: &mut ShooterRuntime) {
    runtime.server = None;
    runtime.advertiser = None;
    runtime.background_hosting = false;
    runtime.local_host_participating = true;
    runtime.config.mode = ShooterMode::Offline;
    runtime.local_id = Some(SERVER_AUTHORITY_ID);
    clear_all_shooter_players(engine);
    clear_transient_gameplay_visuals(engine);
    clear_active_map(engine);
    engine.insert_resource(ShooterSession::default());
    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
        menu.status = "The last player left, so your server closed.".to_string();
    }
}

pub(crate) fn spawn_pause_panel(engine: &mut Engine) {
    engine.spawn_actor("Pause Menu Panel")
        .with(PauseMenuWidget { action: None })
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::splat(0.5), offset_px: Vec2::ZERO, size_px: Vec2::new(420.0, 500.0), z_order: 800 })
        .with(vetrace_ui::UIPanel { size: Vec2::new(420.0, 500.0), background: Vec3::new(0.018, 0.027, 0.042), alpha: 0.97, anchor: Anchor::Center })
        .with(vetrace_ui::UIVisualStyle::rounded(20.0).with_border(1.0, Vec3::new(0.18, 0.38, 0.60), 0.7).with_shadow(Vec2::new(0.0, 12.0), Vec3::ZERO, 0.6))
        .build();
}

pub(crate) fn spawn_pause_label(engine: &mut Engine, text: &str, anchor: Vec2, offset: Vec2, size: Vec2, font_size: f32, color: Vec3) {
    engine.spawn_actor("Pause Menu Label")
        .with(PauseMenuWidget { action: None })
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor, offset_px: offset, size_px: size, z_order: 802 })
        .with(UILabel { text: text.to_string(), font_size, color, anchor: Anchor::Center, align: TextAlign::Center })
        .build();
}

pub(crate) fn spawn_pause_button(engine: &mut Engine, text: &str, action: PauseMenuAction, offset: Vec2, style: vetrace_ui::UIVisualStyle, background: Vec3) {
    spawn_pause_sized_button(engine, text, action, offset, Vec2::new(300.0, 48.0), style, background);
}

pub(crate) fn spawn_pause_sized_button(engine: &mut Engine, text: &str, action: PauseMenuAction, offset: Vec2, size: Vec2, style: vetrace_ui::UIVisualStyle, background: Vec3) {
    engine.spawn_actor("Pause Menu Button")
        .with(PauseMenuWidget { action: Some(action) })
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::splat(0.5), offset_px: offset, size_px: size, z_order: 803 })
        .with(vetrace_ui::UIButton { text: text.to_string(), size, ..vetrace_ui::UIButton::default() })
        .with(style)
        .with(Material { base_color: background, alpha: 0.97, ..Material::default() })
        .build();
}

pub(crate) fn update_pause_menu_pointer(engine: &mut Engine) -> Option<PauseMenuAction> {
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let render = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let viewport = Vec2::new(render.width.max(1) as f32, render.height.max(1) as f32);
    let (x, y) = input.mouse_position();
    let point = Vec2::new(x, y);
    let widgets = engine.actors_with::<PauseMenuWidget>().into_iter().filter_map(|(actor, widget)| {
        Some((actor, widget.action?, actor.get_component::<ScreenSpaceRect>(engine)?.clone()))
    }).collect::<Vec<_>>();
    for (actor, action, rect) in widgets {
        let interaction = vetrace_ui::pointer_interaction(viewport, rect.anchor, rect.offset_px, rect.size_px, point, input.is_mouse_button_down("Left"), input.was_mouse_button_released("Left"));
        if let Some(button) = actor.get_component_mut::<vetrace_ui::UIButton>(engine) {
            button.hovered = interaction.hovered;
            button.pressed = interaction.pressed;
        }
        if interaction.clicked { return Some(action); }
    }
    None
}

pub(crate) fn despawn_pause_menu(engine: &mut Engine) {
    let actors = engine.actors_with::<PauseMenuWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors { actor.despawn(engine); }
}
