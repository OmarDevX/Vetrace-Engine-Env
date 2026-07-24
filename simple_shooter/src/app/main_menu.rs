use super::*;

mod actions;
pub(super) use actions::*;

pub(crate) const MENU_PREVIEW_HIT_SIZE: Vec2 = Vec2::new(300.0, 430.0);

pub(crate) fn setup_main_menu(engine: &mut Engine, runtime: &ShooterRuntime) {
    let game_settings = engine.get_resource::<ShooterGameSettings>().cloned().unwrap_or_default();
    let preview_seed = explicit_player_color_seed(runtime.local_seed);
    engine.insert_resource(MainMenuState {
        active: true,
        page: MainMenuPage::Home,
        color_roll: preview_seed,
        selected_map: if runtime.config.map_json_path.is_some() && map_count() > BUILTIN_MAP_COUNT { BUILTIN_MAP_COUNT } else { 0 },
        selected_bot_difficulty: BotDifficulty::Normal,
        selected_bot_count: runtime.config.bot_count,
        selected_max_players: runtime.config.max_players,
        mod_enabled: false,
        selected_mod: 0,
        vignette_enabled: game_settings.vignette,
        status: "Choose a map, customize your runner, then deploy.".to_string(),
        selected_server: 0,
        server_summary: String::new(),
    });

    if let Some(camera) = engine.get_resource_mut::<Camera>() {
        camera.position = Vec3::new(0.0, 1.65, 6.2);
        camera.target = Vec3::new(0.0, 1.35, 0.0);
        camera.fov_y_radians = 48.0_f32.to_radians();
    }
    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = false;
        settings.cursor_visible = true;
    }

    spawn_main_menu_preview(engine, preview_seed);
    spawn_main_menu_chrome(engine);
    sync_main_menu_post_process(engine, true, game_settings.vignette);
    set_crosshair_visible(engine, false);
}

pub(crate) fn spawn_main_menu_preview(engine: &mut Engine, seed: u64) {
    let shader = PlayerGradientShader::new(0x4d45_4e55, seed);
    engine
        .spawn_actor("Menu Player Preview")
        .with(MainMenuPreviewPlayer)
        .with(Transform {
            translation: Vec3::new(0.0, 1.42, 0.0),
            rotation: Quat::from_rotation_y(-0.35),
            scale: Vec3::ONE,
        })
        // Match the real Simple Shooter player visual instead of presenting a
        // capsule that is only representative of character collision shapes.
        .with(Shape {
            primitive: PrimitiveShape::Cube,
            size: Vec3::new(PLAYER_RADIUS * 2.0, PLAYER_VISUAL_HEIGHT, PLAYER_RADIUS * 2.0),
        })
        .with(Material { base_color: shader.color_a, roughness: 0.28, metallic: 0.08, ..Material::default() })
        .with(shader)
        .with(player_gradient_material(shader, 1.0))
        .with(Renderable { visible: true, ..Renderable::default() })
        .with(ShooterOutlineStyle::default())
        .build();

    let outline_style = ShooterOutlineStyle::default();
    engine.spawn_actor("Menu Player Preview Outline")
        .with(MainMenuPreviewOutline)
        .with(Transform { translation: Vec3::new(0.0, 1.42, 0.0), rotation: Quat::from_rotation_y(-0.35), scale: Vec3::ONE })
        .with(player_outline_shape(outline_style, Vec3::ONE))
        .with(Material { base_color: outline_style.color, alpha: 1.0, ..Material::default() })
        .with(player_outline_material(outline_style))
        .with(Renderable { visible: true, ..Renderable::default() })
        .build();

    engine
        .spawn_actor("Menu Preview Stage")
        .with(MainMenuPreviewStage)
        .with(Transform {
            translation: Vec3::new(0.0, 0.02, 0.0),
            scale: Vec3::ONE,
            ..Transform::default()
        })
        .with(Shape { primitive: PrimitiveShape::Cube, size: Vec3::new(3.6, 0.12, 3.6) })
        .with(Material {
            base_color: Vec3::new(0.035, 0.045, 0.065),
            emissive: Vec3::new(0.025, 0.09, 0.15),
            metallic: 0.7,
            roughness: 0.25,
            ..Material::default()
        })
        .with(Renderable { visible: true, ..Renderable::default() })
        .build();

    // An invisible vetrace_ui hit target overlays the projected character.
    spawn_menu_hit_target(engine, MainMenuAction::RandomizePlayer, Vec2::new(0.5, 0.5), Vec2::new(0.0, -12.0), MENU_PREVIEW_HIT_SIZE);
}

pub(crate) fn spawn_main_menu_chrome(engine: &mut Engine) {
    spawn_menu_label(
        engine,
        "VETRACE // ARENA",
        Vec2::new(0.0, 0.0),
        Vec2::new(184.0, 54.0),
        Vec2::new(330.0, 46.0),
        30.0,
        Vec3::new(0.96, 0.98, 1.0),
        false,
    );
    spawn_menu_label(
        engine,
        "LOADOUT PREVIEW  •  CLICK PLAYER TO REROLL COLOR",
        Vec2::new(0.5, 1.0),
        Vec2::new(0.0, -35.0),
        Vec2::new(540.0, 28.0),
        13.0,
        Vec3::new(0.55, 0.66, 0.78),
        false,
    );

    spawn_menu_button(engine, "SERVERS", MainMenuAction::Servers, Vec2::new(0.5, 0.0), Vec2::new(-245.0, 55.0), Vec2::new(135.0, 42.0), menu_dark_button_style(), Vec3::new(0.045, 0.065, 0.09), false);
    spawn_menu_button(engine, "MODS", MainMenuAction::Mods, Vec2::new(0.5, 0.0), Vec2::new(-95.0, 55.0), Vec2::new(130.0, 42.0), menu_dark_button_style(), Vec3::new(0.045, 0.065, 0.09), false);
    spawn_menu_button(engine, "SETTINGS", MainMenuAction::Settings, Vec2::new(0.5, 0.0), Vec2::new(70.0, 55.0), Vec2::new(160.0, 42.0), menu_dark_button_style(), Vec3::new(0.045, 0.065, 0.09), false);

    spawn_menu_button(engine, "MAPS", MainMenuAction::Maps, Vec2::new(1.0, 1.0), Vec2::new(-155.0, -145.0), Vec2::new(230.0, 48.0), menu_dark_button_style(), Vec3::new(0.055, 0.075, 0.105), false);
    spawn_menu_button(engine, "PLAY", MainMenuAction::Play, Vec2::new(1.0, 1.0), Vec2::new(-155.0, -78.0), Vec2::new(230.0, 64.0), menu_play_button_style(), Vec3::new(0.96, 0.70, 0.04), false);
}

pub(crate) fn update_main_menu(engine: &mut Engine, runtime: &mut ShooterRuntime, dt: f32) -> bool {
    let active = engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false);
    if !active { return false; }

    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = false;
        settings.cursor_visible = true;
    }
    if engine.get_resource::<InputState>().map(|input| input.quit_requested()).unwrap_or(false) {
        engine.stop();
        return true;
    }

    animate_main_menu_preview(engine, runtime.time, dt);
    let servers_page = engine.get_resource::<MainMenuState>().map(|menu| menu.page == MainMenuPage::Servers).unwrap_or(false);
    if servers_page {
        let summary = server_page_summary(runtime);
        let changed = engine.get_resource::<MainMenuState>().map(|menu| menu.server_summary != summary).unwrap_or(false);
        if changed { show_servers_page(engine, runtime); }
    }
    let preview_transform = engine.actors_with::<MainMenuPreviewPlayer>().into_iter().next()
        .and_then(|(actor, _)| actor.get_component::<Transform>(engine).cloned());
    if let Some(preview_transform) = preview_transform {
        let outlines = engine.actors_with::<MainMenuPreviewOutline>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
        for actor in outlines { let _ = actor.insert(engine, preview_transform.clone()); }
    }
    let clicked = update_main_menu_pointer(engine);
    if let Some(action) = clicked {
        apply_main_menu_action(engine, runtime, action);
    }

    let (active, vignette) = engine.get_resource::<MainMenuState>()
        .map(|menu| (menu.active, menu.vignette_enabled))
        .unwrap_or((false, false));
    let post_enabled = if active {
        vignette
    } else {
        engine.get_resource::<ShooterGameSettings>().map(|settings| settings.vignette).unwrap_or(runtime.config.post_vignette)
    };
    sync_main_menu_post_process(engine, active, post_enabled);
    active
}

pub(crate) fn animate_main_menu_preview(engine: &mut Engine, time: f32, dt: f32) {
    let actors = engine.actors_with::<MainMenuPreviewPlayer>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in actors {
        if let Some(transform) = actor.get_component_mut::<Transform>(engine) {
            let yaw = Quat::from_rotation_y(dt * 0.55);
            transform.rotation = yaw * transform.rotation;
            transform.translation.y = 1.42 + (time * 1.8).sin() * 0.025;
        }
    }
}

pub(crate) fn update_main_menu_pointer(engine: &mut Engine) -> Option<MainMenuAction> {
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let viewport = Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32);
    let (mx, my) = input.mouse_position();
    let point = Vec2::new(mx, my);
    let pointer_down = input.is_mouse_button_down("Left");
    let pointer_released = input.was_mouse_button_released("Left");
    let mut widgets = engine.actors_with::<MainMenuWidget>()
        .into_iter()
        .filter_map(|(actor, widget)| {
            let action = widget.action?;
            let rect = actor.get_component::<ScreenSpaceRect>(engine)?.clone();
            Some((actor, action, rect))
        })
        .collect::<Vec<_>>();
    widgets.sort_by(|a, b| b.2.z_order.cmp(&a.2.z_order));

    let mut clicked = None;
    for (actor, action, rect) in widgets {
        let interaction = vetrace_ui::pointer_interaction(
            viewport,
            rect.anchor,
            rect.offset_px,
            rect.size_px,
            point,
            pointer_down,
            pointer_released,
        );
        if let Some(button) = actor.get_component_mut::<vetrace_ui::UIButton>(engine) {
            button.hovered = interaction.hovered;
            button.pressed = interaction.pressed;
        }
        if interaction.clicked && clicked.is_none() {
            clicked = Some(action);
        }
    }
    clicked
}

mod settings;

pub(crate) fn show_main_menu_page(engine: &mut Engine, page: MainMenuPage) {
    despawn_main_menu_page(engine);
    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
        menu.page = page;
    }
    if page == MainMenuPage::Home { return; }

    spawn_menu_panel(engine, Vec2::new(0.0, 0.5), Vec2::new(310.0, 35.0), Vec2::new(510.0, 410.0), true);
    let (title, body) = {
        let menu = engine.get_resource::<MainMenuState>().expect("main menu state");
        match page {
            MainMenuPage::Servers => unreachable!(),
            MainMenuPage::Maps => ("SELECT MAP", format!("{}\n\nBOTS: {}   •   DIFFICULTY: {}\nMAX HUMAN PLAYERS: {}\nCounts are automatically limited by map spawn capacity", map_name(menu.selected_map as u8), menu.selected_bot_count, menu.selected_bot_difficulty.name(), menu.selected_max_players)),
            MainMenuPage::Shop => ("SHOP", "FEATURED LOADOUTS\n\nSolar Flare      1,200 CR\nMidnight Runner    800 CR\nFounder Pack     OWNED\n\nPurchases are presentation-only in this demo.".to_string()),
            MainMenuPage::Mods => ("LUA MODS", shooter_mod_page_text(engine, menu.selected_mod)),
            MainMenuPage::Settings => {
                let settings = engine.get_resource::<ShooterGameSettings>().cloned().unwrap_or_default();
                ("SETTINGS", format!("GRAPHICS  {:?}    •    VSYNC {}\nFOG {}    •    VIGNETTE {}\n\nMOUSE SENSITIVITY  {:.1}x\nMASTER VOLUME      {:>3}%\n\nChanges save automatically.", settings.graphics_profile, if settings.vsync { "ON" } else { "OFF" }, if settings.volumetric_fog { "ON" } else { "OFF" }, if settings.vignette { "ON" } else { "OFF" }, settings.mouse_sensitivity / FPS_MOUSE_SENSITIVITY, (settings.master_volume * 100.0).round() as u32))
            },
            MainMenuPage::Home => unreachable!(),
        }
    };
    spawn_menu_label(engine, title, Vec2::new(0.0, 0.5), Vec2::new(310.0, -125.0), Vec2::new(420.0, 44.0), 25.0, Vec3::ONE, true);
    spawn_menu_label(engine, &body, Vec2::new(0.0, 0.5), Vec2::new(310.0, 0.0), Vec2::new(420.0, 210.0), 16.0, Vec3::new(0.72, 0.79, 0.88), true);
    spawn_menu_button(engine, "×", MainMenuAction::ClosePage, Vec2::new(0.0, 0.5), Vec2::new(520.0, -135.0), Vec2::new(42.0, 42.0), menu_dark_button_style(), Vec3::new(0.08, 0.10, 0.14), true);

    match page {
        MainMenuPage::Maps => {
            spawn_menu_button(engine, "MAP ‹", MainMenuAction::PreviousMap, Vec2::new(0.0, 0.5), Vec2::new(175.0, 150.0), Vec2::new(105.0, 44.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "MAP ›", MainMenuAction::NextMap, Vec2::new(0.0, 0.5), Vec2::new(290.0, 150.0), Vec2::new(105.0, 44.0), menu_accent_button_style(), Vec3::new(0.08, 0.42, 0.72), true);
            spawn_menu_button(engine, "DIFF ‹", MainMenuAction::PreviousBotDifficulty, Vec2::new(0.0, 0.5), Vec2::new(405.0, 150.0), Vec2::new(105.0, 44.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "DIFF ›", MainMenuAction::NextBotDifficulty, Vec2::new(0.0, 0.5), Vec2::new(520.0, 150.0), Vec2::new(105.0, 44.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "BOTS −", MainMenuAction::BotCountDown, Vec2::new(0.0, 0.5), Vec2::new(205.0, 100.0), Vec2::new(105.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "BOTS +", MainMenuAction::BotCountUp, Vec2::new(0.0, 0.5), Vec2::new(320.0, 100.0), Vec2::new(105.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "PLAYERS −", MainMenuAction::MaxPlayersDown, Vec2::new(0.0, 0.5), Vec2::new(435.0, 100.0), Vec2::new(105.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "PLAYERS +", MainMenuAction::MaxPlayersUp, Vec2::new(0.0, 0.5), Vec2::new(550.0, 100.0), Vec2::new(105.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
        }
        MainMenuPage::Mods => {
            spawn_menu_button(engine, "‹", MainMenuAction::PreviousMod, Vec2::new(0.0, 0.5), Vec2::new(130.0, 150.0), Vec2::new(55.0, 44.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "ENABLE / DISABLE", MainMenuAction::ToggleMod, Vec2::new(0.0, 0.5), Vec2::new(270.0, 150.0), Vec2::new(205.0, 44.0), menu_accent_button_style(), Vec3::new(0.26, 0.48, 0.84), true);
            spawn_menu_button(engine, "RELOAD", MainMenuAction::ReloadMod, Vec2::new(0.0, 0.5), Vec2::new(415.0, 150.0), Vec2::new(90.0, 44.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "›", MainMenuAction::NextMod, Vec2::new(0.0, 0.5), Vec2::new(490.0, 150.0), Vec2::new(55.0, 44.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
        }
        MainMenuPage::Settings => {
            spawn_menu_button(engine, "QUALITY", MainMenuAction::CycleGraphics, Vec2::new(0.0, 0.5), Vec2::new(105.0, 105.0), Vec2::new(95.0, 40.0), menu_accent_button_style(), Vec3::new(0.26, 0.48, 0.84), true);
            spawn_menu_button(engine, "VSYNC", MainMenuAction::ToggleVsync, Vec2::new(0.0, 0.5), Vec2::new(205.0, 105.0), Vec2::new(95.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "FOG", MainMenuAction::ToggleVolumetricFog, Vec2::new(0.0, 0.5), Vec2::new(305.0, 105.0), Vec2::new(95.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "VIGNETTE", MainMenuAction::ToggleVignette, Vec2::new(0.0, 0.5), Vec2::new(430.0, 105.0), Vec2::new(145.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "SENS −", MainMenuAction::SensitivityDown, Vec2::new(0.0, 0.5), Vec2::new(125.0, 155.0), Vec2::new(115.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "SENS +", MainMenuAction::SensitivityUp, Vec2::new(0.0, 0.5), Vec2::new(250.0, 155.0), Vec2::new(115.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "VOL −", MainMenuAction::VolumeDown, Vec2::new(0.0, 0.5), Vec2::new(375.0, 155.0), Vec2::new(105.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "VOL +", MainMenuAction::VolumeUp, Vec2::new(0.0, 0.5), Vec2::new(485.0, 155.0), Vec2::new(105.0, 40.0), menu_dark_button_style(), Vec3::new(0.06, 0.09, 0.13), true);
            spawn_menu_button(engine, "RESET", MainMenuAction::ResetSettings, Vec2::new(0.0, 0.5), Vec2::new(485.0, -95.0), Vec2::new(105.0, 36.0), menu_dark_button_style(), Vec3::new(0.11, 0.07, 0.08), true);
        }
        MainMenuPage::Servers | MainMenuPage::Shop | MainMenuPage::Home => {}
    }
}

mod server_page;
mod widgets;

pub(super) use server_page::*;
pub(super) use settings::*;
pub(super) use widgets::*;
