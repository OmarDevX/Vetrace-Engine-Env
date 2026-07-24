use super::*;

pub(crate) fn randomize_main_menu_player(engine: &mut Engine) {
    let seed = {
        let Some(menu) = engine.get_resource_mut::<MainMenuState>() else { return; };
        let next = menu.color_roll.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        menu.color_roll = explicit_player_color_seed(next);
        menu.status = "Generated a new gradient loadout".to_string();
        menu.color_roll
    };
    let Some(actor) = engine.actors_with::<MainMenuPreviewPlayer>().into_iter().next().map(|(actor, _)| actor) else { return; };
    let shader = PlayerGradientShader::new(0x4d45_4e55, seed);
    let _ = actor.insert(engine, shader);
    let _ = actor.insert(engine, player_gradient_material(shader, 1.0));
    if let Some(material) = actor.get_component_mut::<Material>(engine) { material.base_color = shader.color_a; }
}

pub(crate) fn update_game_settings(engine: &mut Engine, change: impl FnOnce(&mut ShooterGameSettings)) {
    if let Some(settings) = engine.get_resource_mut::<ShooterGameSettings>() {
        change(settings);
        *settings = settings.clone().normalized();
    }
    apply_and_save_game_settings(engine);
    if engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false) { show_main_menu_page(engine, MainMenuPage::Settings); }
}

pub(crate) fn apply_and_save_game_settings(engine: &mut Engine) {
    let settings = engine.get_resource::<ShooterGameSettings>().cloned().unwrap_or_default().normalized();
    if let Some(saved) = engine.get_resource_mut::<ShooterGameSettings>() { *saved = settings.clone(); }
    if let Some(menu) = engine.get_resource_mut::<MainMenuState>() {
        menu.vignette_enabled = settings.vignette;
        menu.status = settings.save().map(|_| "Settings applied and saved".to_string()).unwrap_or_else(|error| format!("Settings applied; save failed: {error}"));
    } else if let Err(error) = settings.save() { eprintln!("Simple Shooter settings: could not save: {error}"); }
    if let Some(render) = engine.get_resource_mut::<RenderSettings>() {
        settings.graphics_profile.apply_to_render_settings(render);
        render.present_mode = if settings.vsync { PresentModePreference::Vsync } else { PresentModePreference::LowLatency };
    }
    if let Some(baked) = engine.get_resource_mut::<ShooterBakedLightingSettings>() {
        baked.graphics_profile = settings.graphics_profile;
    }
    set_baked_lighting_runtime_mode(
        engine,
        match settings.graphics_profile {
            ShooterGraphicsProfile::HighQuality => BakedLightingRuntimeMode::HybridRealtimeDirect,
            ShooterGraphicsProfile::LowSpec | ShooterGraphicsProfile::Balanced => BakedLightingRuntimeMode::BakedOnly,
        },
    );
    let shadow_mode = match settings.graphics_profile { ShooterGraphicsProfile::LowSpec => ShadowMode::None, ShooterGraphicsProfile::Balanced => ShadowMode::Hard, ShooterGraphicsProfile::HighQuality => ShadowMode::Soft };
    for actor in engine.actors_with::<DirectionalLight>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>() {
        if let Some(light) = actor.get_component_mut::<DirectionalLight>(engine) { light.shadow_mode = shadow_mode; }
    }
    for actor in engine.actors_with::<VolumetricFog>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>() {
        if let Some(fog) = actor.get_component_mut::<VolumetricFog>(engine) { fog.enabled = settings.volumetric_fog; }
    }
    apply_live_game_settings(engine);
    let menu_active = engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false);
    sync_main_menu_post_process(engine, menu_active, settings.vignette);
}

pub(crate) fn apply_live_game_settings(engine: &mut Engine) {
    let settings = engine.get_resource::<ShooterGameSettings>().cloned().unwrap_or_default().normalized();
    for actor in engine.actors_with::<FirstPersonController>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>() {
        if let Some(controller) = actor.get_component_mut::<FirstPersonController>(engine) { controller.mouse_sensitivity = settings.mouse_sensitivity; }
    }
    for actor in engine.actors_with::<FreeFlightController>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>() {
        if let Some(controller) = actor.get_component_mut::<FreeFlightController>(engine) { controller.sensitivity = settings.mouse_sensitivity; }
    }
    #[cfg(feature = "audio")]
    let audio_actors = engine.actors_with::<AudioSource>().into_iter().map(|(actor, source)| (actor, source.looping && !source.spatial)).collect::<Vec<_>>();
    #[cfg(feature = "audio")]
    for (actor, is_music) in audio_actors {
        if let Some(source) = actor.get_component_mut::<AudioSource>(engine) { source.volume = if is_music { 0.45 } else { 0.95 } * settings.master_volume; }
    }
}
