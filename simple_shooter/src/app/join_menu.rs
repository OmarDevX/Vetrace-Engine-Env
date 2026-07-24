use super::*;

pub(crate) const JOIN_PANEL_SIZE: Vec2 = Vec2::new(440.0, 250.0);
pub(crate) const JOIN_TITLE_OFFSET: Vec2 = Vec2::new(0.0, -88.0);
pub(crate) const JOIN_FIELD_OFFSET: Vec2 = Vec2::new(0.0, -20.0);
pub(crate) const JOIN_BUTTON_OFFSET: Vec2 = Vec2::new(0.0, 48.0);
pub(crate) const JOIN_HINT_OFFSET: Vec2 = Vec2::new(0.0, 96.0);
pub(crate) const JOIN_FIELD_SIZE: Vec2 = Vec2::new(320.0, 42.0);
pub(crate) const JOIN_BUTTON_SIZE: Vec2 = Vec2::new(180.0, 42.0);
pub(crate) const JOIN_HINT_SIZE: Vec2 = Vec2::new(380.0, 32.0);

pub(crate) fn setup_join_menu_ui(engine: &mut Engine, config: &ShooterConfig) {
    if !config.prompt_player_name_in_ui {
        return;
    }

    engine.insert_resource(ShooterJoinMenu {
        active: true,
        name: String::new(),
        focused: true,
        status: "Type your player name, then click Join or press Enter.".to_string(),
    });

    spawn_join_panel(engine);
    spawn_join_title(engine);
    spawn_join_name_field(engine, &config.player_name);
    spawn_join_button(engine);
    spawn_join_hint(engine);
}

pub(crate) fn update_join_menu(engine: &mut Engine, runtime: &mut ShooterRuntime) -> bool {
    let active = engine
        .get_resource::<ShooterJoinMenu>()
        .map(|menu| menu.active)
        .unwrap_or(false);
    if !active {
        return false;
    }

    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let (mouse_x, mouse_y) = input.mouse_position();
    let mouse = Vec2::new(mouse_x, mouse_y);
    let field_hovered = screen_rect_contains(&settings, JOIN_FIELD_OFFSET, JOIN_FIELD_SIZE, mouse);
    let button_hovered = screen_rect_contains(&settings, JOIN_BUTTON_OFFSET, JOIN_BUTTON_SIZE, mouse);
    if input.quit_requested() || input.was_key_pressed("Escape") {
        engine.stop();
        return true;
    }

    let mouse_pressed = input.was_mouse_button_pressed("Left");
    let mouse_released = input.was_mouse_button_released("Left");
    let button_pressed = button_hovered && input.is_mouse_button_down("Left");

    let mut submitted = false;
    let mut submitted_name = String::new();
    {
        let Some(menu) = engine.get_resource_mut::<ShooterJoinMenu>() else { return false; };
        if mouse_pressed {
            menu.focused = field_hovered;
        }

        if menu.focused {
            for ch in input.text_input().chars() {
                if menu.name.chars().count() >= 24 {
                    break;
                }
                if !ch.is_control() {
                    menu.name.push(ch);
                }
            }
            if input.was_key_pressed("Backspace") {
                menu.name.pop();
            }
        }

        let enter_submit = input.was_key_pressed("Enter");
        let click_submit = button_hovered && mouse_released;
        if enter_submit || click_submit {
            let clean = sanitize_player_name(&menu.name);
            if clean.trim().is_empty() {
                menu.status = "Name cannot be empty.".to_string();
                menu.focused = true;
            } else {
                menu.name = clean.clone();
                menu.status = format!("Joining as {clean}...");
                submitted = true;
                submitted_name = clean;
            }
        }
    }

    sync_join_menu_widgets(engine, field_hovered, button_hovered, button_pressed);

    if submitted {
        finish_join_menu(engine, runtime, submitted_name);
        return false;
    }

    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = false;
        settings.cursor_visible = true;
    }

    true
}

pub(crate) fn finish_join_menu(engine: &mut Engine, runtime: &mut ShooterRuntime, name: String) {
    let clean = sanitize_player_name(&name);
    runtime.config.player_name = clean.clone();
    // A main-menu preview selection already established the player's visual
    // seed. Preserve it; only derive a seed from the name when the front end
    // was bypassed and no visual selection took place.
    if !runtime.config.show_main_menu {
        runtime.local_seed = automatic_player_color_seed(stable_hash(&clean));
    }
    runtime.name_ready = true;
    if let Some(advertiser) = runtime.advertiser.as_mut() {
        advertiser.name = format!("{}'s server", clean);
    }
    if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
        if session.local_is_admin { session.server_name = format!("{}'s server", clean); }
    }
    if let Some(client) = runtime.client.as_mut() {
        client.name = clean.clone();
        client.color_seed = runtime.local_seed;
        client.net.set_hello(ShooterHello { name: client.name.clone(), color_seed: client.color_seed });
    }
    if let Some(menu) = engine.get_resource_mut::<ShooterJoinMenu>() {
        menu.active = false;
        menu.name = clean.clone();
        menu.status = format!("Joined as {clean}");
    }
    despawn_join_menu_widgets(engine);
    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = !runtime.editor_enabled;
        settings.cursor_visible = runtime.editor_enabled;
    }
    println!("player name: {clean}");
}

pub(crate) fn spawn_join_panel(engine: &mut Engine) -> Actor {
    engine
        .spawn_actor("Join Menu Panel")
        .with(ShooterJoinMenuWidget { role: ShooterJoinMenuRole::Panel })
        .with(vetrace_ui::UIScreenSpace)
        .with(screen_rect(Vec2::ZERO, JOIN_PANEL_SIZE, 100))
        .with(vetrace_ui::UIPanel {
            size: JOIN_PANEL_SIZE,
            background: Vec3::new(0.04, 0.045, 0.055),
            alpha: 0.88,
            anchor: Anchor::Center,
        })
        .with(vetrace_ui::UIVisualStyle::rounded(18.0)
            .with_border(1.0, Vec3::new(0.16, 0.36, 0.58), 0.7)
            .with_shadow(Vec2::new(0.0, 10.0), Vec3::ZERO, 0.55))
        .with(Material { base_color: Vec3::new(0.04, 0.045, 0.055), alpha: 0.88, ..Material::default() })
        .build()
}

pub(crate) fn spawn_join_title(engine: &mut Engine) -> Actor {
    engine
        .spawn_actor("Join Menu Title")
        .with(ShooterJoinMenuWidget { role: ShooterJoinMenuRole::Title })
        .with(vetrace_ui::UIScreenSpace)
        .with(screen_rect(JOIN_TITLE_OFFSET, Vec2::new(380.0, 36.0), 101))
        .with(UILabel {
            text: "Simple Shooter".to_string(),
            font_size: 24.0,
            color: Vec3::ONE,
            anchor: Anchor::Center,
            align: TextAlign::Center,
        })
        .with(Material { alpha: 0.0, ..Material::default() })
        .build()
}

pub(crate) fn spawn_join_name_field(engine: &mut Engine, _default_name: &str) -> Actor {
    engine
        .spawn_actor("Join Menu Name Field")
        .with(ShooterJoinMenuWidget { role: ShooterJoinMenuRole::NameField })
        .with(vetrace_ui::UIScreenSpace)
        .with(screen_rect(JOIN_FIELD_OFFSET, JOIN_FIELD_SIZE, 102))
        .with(vetrace_ui::UITextEditor {
            text: String::new(),
            placeholder: "Player name".to_string(),
            focused: true,
            multiline: false,
        })
        .with(vetrace_ui::UIVisualStyle::rounded(10.0)
            .with_border(1.0, Vec3::new(0.20, 0.44, 0.72), 0.8))
        .with(Material { base_color: Vec3::new(0.10, 0.11, 0.13), alpha: 0.92, ..Material::default() })
        .build()
}

pub(crate) fn spawn_join_button(engine: &mut Engine) -> Actor {
    engine
        .spawn_actor("Join Menu Button")
        .with(ShooterJoinMenuWidget { role: ShooterJoinMenuRole::JoinButton })
        .with(vetrace_ui::UIScreenSpace)
        .with(screen_rect(JOIN_BUTTON_OFFSET, JOIN_BUTTON_SIZE, 103))
        .with(vetrace_ui::UIButton {
            text: "Join".to_string(),
            size: JOIN_BUTTON_SIZE,
            hovered: false,
            pressed: false,
            enabled: true,
        })
        .with({
            let mut style = vetrace_ui::UIVisualStyle::rounded(12.0)
                .with_border(1.0, Vec3::new(0.32, 0.60, 1.0), 0.85)
                .with_shadow(Vec2::new(0.0, 5.0), Vec3::ZERO, 0.45);
            style.font_size = 18.0;
            style
        })
        .with(Material { base_color: Vec3::new(0.12, 0.30, 0.78), alpha: 0.95, ..Material::default() })
        .build()
}

pub(crate) fn spawn_join_hint(engine: &mut Engine) -> Actor {
    engine
        .spawn_actor("Join Menu Hint")
        .with(ShooterJoinMenuWidget { role: ShooterJoinMenuRole::Hint })
        .with(vetrace_ui::UIScreenSpace)
        .with(screen_rect(JOIN_HINT_OFFSET, JOIN_HINT_SIZE, 104))
        .with(UILabel {
            text: "Click Join or press Enter".to_string(),
            font_size: 15.0,
            color: Vec3::new(0.82, 0.84, 0.90),
            anchor: Anchor::Center,
            align: TextAlign::Center,
        })
        .with(Material { alpha: 0.0, ..Material::default() })
        .build()
}

pub(crate) fn sync_join_menu_widgets(engine: &mut Engine, field_hovered: bool, button_hovered: bool, button_pressed: bool) {
    let menu = engine.get_resource::<ShooterJoinMenu>().cloned();
    let Some(menu) = menu else { return; };
    let widgets = engine.actors_with::<ShooterJoinMenuWidget>()
        .into_iter()
        .map(|(actor, widget)| (actor, widget.role))
        .collect::<Vec<_>>();

    for (actor, role) in widgets {
        match role {
            ShooterJoinMenuRole::NameField => {
                if let Some(editor) = actor.get_component_mut::<vetrace_ui::UITextEditor>(engine) {
                    editor.text = menu.name.clone();
                    editor.focused = menu.focused;
                }
                if let Some(material) = actor.get_component_mut::<Material>(engine) {
                    material.base_color = if menu.focused {
                        Vec3::new(0.13, 0.15, 0.20)
                    } else if field_hovered {
                        Vec3::new(0.12, 0.13, 0.16)
                    } else {
                        Vec3::new(0.10, 0.11, 0.13)
                    };
                }
            }
            ShooterJoinMenuRole::JoinButton => {
                if let Some(button) = actor.get_component_mut::<vetrace_ui::UIButton>(engine) {
                    button.hovered = button_hovered;
                    button.pressed = button_pressed;
                    button.enabled = true;
                }
                if let Some(material) = actor.get_component_mut::<Material>(engine) {
                    material.base_color = if button_pressed {
                        Vec3::new(0.08, 0.20, 0.58)
                    } else if button_hovered {
                        Vec3::new(0.16, 0.38, 0.92)
                    } else {
                        Vec3::new(0.12, 0.30, 0.78)
                    };
                }
            }
            ShooterJoinMenuRole::Hint => {
                if let Some(label) = actor.get_component_mut::<UILabel>(engine) {
                    label.text = menu.status.clone();
                }
            }
            ShooterJoinMenuRole::Panel | ShooterJoinMenuRole::Title => {}
        }
    }
}

pub(crate) fn despawn_join_menu_widgets(engine: &mut Engine) {
    let actors = engine.actors_with::<ShooterJoinMenuWidget>()
        .into_iter()
        .map(|(actor, _)| actor)
        .collect::<Vec<_>>();
    for actor in actors {
        actor.despawn(engine);
    }
}

pub(crate) fn screen_rect(offset_px: Vec2, size_px: Vec2, z_order: i32) -> ScreenSpaceRect {
    ScreenSpaceRect {
        anchor: Vec2::new(0.5, 0.5),
        offset_px,
        size_px,
        z_order,
    }
}

pub(crate) fn screen_rect_contains(settings: &RenderSettings, offset_px: Vec2, size_px: Vec2, point: Vec2) -> bool {
    vetrace_ui::screen_rect_contains(
        Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32),
        Vec2::new(0.5, 0.5),
        offset_px,
        size_px,
        point,
    )
}
