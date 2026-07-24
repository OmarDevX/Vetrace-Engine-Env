use super::*;

pub(crate) fn register_components(engine: &mut Engine) {
    if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
        cm.register_named::<ShooterPlayer>("simple_shooter.player", "Shooter Player");
        cm.register_named::<FirstPersonController>("simple_shooter.first_person_controller", "First Person Controller");
        cm.register_named::<FreeFlightController>("simple_shooter.free_flight_controller", "Free Flight Controller");
        cm.register_named::<LocalPlayer>("simple_shooter.local_player", "Local Player");
        cm.register_named::<RemotePlayer>("simple_shooter.remote_player", "Remote Player");
        cm.register_named::<PlayerNameLabel>("simple_shooter.player_name_label", "Player Name Label");
        cm.register_named::<ShooterJoinMenuWidget>("simple_shooter.join_menu_widget", "Join Menu Widget");
        cm.register_named::<MainMenuWidget>("simple_shooter.main_menu_widget", "Main Menu Widget");
        cm.register_named::<MainMenuPreviewPlayer>("simple_shooter.main_menu_preview_player", "Main Menu Preview Player");
        cm.register_named::<MainMenuPreviewStage>("simple_shooter.main_menu_preview_stage", "Main Menu Preview Stage");
        cm.register_named::<MainMenuPreviewOutline>("simple_shooter.main_menu_preview_outline", "Main Menu Preview Outline");
        cm.register_named::<PauseMenuWidget>("simple_shooter.pause_menu_widget", "Pause Menu Widget");
        cm.register_named::<LobbyWidget>("simple_shooter.lobby_widget", "Lobby Widget");
        cm.register_named::<LeaderboardWidget>("simple_shooter.leaderboard_widget", "Leaderboard Widget");
        cm.register_named::<RoundResultsWidget>("simple_shooter.round_results_widget", "Round Results Widget");
        cm.register_named::<KillcamWidget>("simple_shooter.killcam_widget", "Killcam Widget");
        cm.register_named::<HealthHudWidget>("simple_shooter.health_hud_widget", "Health HUD Widget");
        cm.register_named::<ShooterOutlineStyle>("simple_shooter.outline_style", "Shooter Outline Style");
        cm.register_named::<ShooterOutlineShell>("simple_shooter.outline_shell", "Shooter Outline Shell");
        cm.register_named::<ShooterOutlineOwner>("simple_shooter.outline_owner", "Shooter Outline Owner");
        cm.register_named::<PlayerGradientShader>("simple_shooter.player_gradient", "Player Gradient Shader");
        cm.register_named::<ShooterInput>("simple_shooter.input", "Shooter Input");
        cm.register_named::<BulletTrail>("simple_shooter.bullet_trail", "Bullet Trail");
        cm.register_named::<WeaponAttachment>("simple_shooter.weapon_attachment", "Weapon Attachment");
        cm.register_named::<WeaponPart>("simple_shooter.weapon_part", "Weapon Part");
        cm.register_named::<MuzzleFlash>("simple_shooter.muzzle_flash", "Muzzle Flash");
        cm.register_named::<EquippedWeapon>("simple_shooter.equipped_weapon", "Equipped Weapon");
        cm.register_named::<PlayerVisualOwner>("simple_shooter.player_visual_owner", "Player Visual Owner");
        cm.register_named::<ShooterStats>("simple_shooter.stats", "Shooter Stats");
        cm.register_named::<ShooterBot>("simple_shooter.bot", "Shooter Bot");
        cm.register_named::<ShooterMapGeometry>("simple_shooter.map_geometry", "Shooter Map Geometry");
        cm.register_named::<ShooterNavigationObstacle>("simple_shooter.navigation_obstacle", "Shooter Navigation Obstacle");
        cm.register_named::<ShooterBotNavigation>("simple_shooter.bot_navigation", "Shooter Bot Navigation");
        cm.register_named::<CrosshairPart>("simple_shooter.crosshair_part", "Crosshair Part");
        cm.register_named::<ShooterAudioListener>("simple_shooter.audio_listener", "Shooter Audio Listener");
    }
}

pub(crate) fn setup_render_resources(engine: &mut Engine, config: &ShooterConfig) {
    let render_settings = shooter_initial_render_settings(config);
    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        *settings = render_settings;
    } else {
        engine.insert_resource(render_settings);
    }

    engine.insert_resource(Camera {
        position: Vec3::new(0.0, FPS_EYE_HEIGHT, 6.0),
        target: Vec3::new(0.0, FPS_EYE_HEIGHT, 5.0),
        fov_y_radians: 75.0_f32.to_radians(),
        ..Camera::default()
    });
    engine.insert_resource(ShooterKillcamState::default());
}

pub(crate) fn setup_world(engine: &mut Engine, config: &ShooterConfig) {
    engine.insert_resource(ShooterBakedLightingSettings {
        force_bake: config.bake_lighting,
        graphics_profile: config.graphics_profile,
    });
    let profile_shadow_mode = match config.graphics_profile {
        ShooterGraphicsProfile::LowSpec => ShadowMode::None,
        ShooterGraphicsProfile::Balanced => ShadowMode::Hard,
        ShooterGraphicsProfile::HighQuality => ShadowMode::Soft,
    };
    let shadow_mode = match config.force_shadows {
        Some(false) => ShadowMode::None,
        Some(true) if profile_shadow_mode == ShadowMode::None => ShadowMode::Hard,
        Some(true) => profile_shadow_mode,
        None => profile_shadow_mode,
    };
    engine
        .spawn_actor("Sun Light")
        .with(DirectionalLight {
            direction: Vec3::new(-0.4, -1.0, -0.3),
            intensity: 2.0,
            shadow_mode,
            ..DirectionalLight::default()
        })
        .build();

    engine
        .spawn_actor("Atmosphere")
        .with(Atmosphere::default())
        .with(VolumetricFog {
            enabled: config.force_fog.unwrap_or_else(|| config.graphics_profile.enables_demo_fog_by_default()),
            density: 0.045,
            anisotropy: 0.35,
            color: Vec3::new(0.25, 0.31, 0.38),
            ..VolumetricFog::default()
        })
        .build();

    setup_custom_post_processes(engine, config);
    spawn_crosshair(engine);
    spawn_shooter_audio(engine);
    engine.insert_resource(ShooterStats::default());
    let mut session = ShooterSession::default();
    session.rules.bot_count = config.bot_count;
    session.rules.bots_enabled = config.bot_count > 0;
    session.rules.max_players = config.max_players;
    if config.map_json_path.is_some() && map_count() > BUILTIN_MAP_COUNT {
        session.rules.map_index = BUILTIN_MAP_COUNT as u8;
    }
    if !config.show_main_menu && !matches!(config.mode, ShooterMode::Offline) {
        session.phase = MatchPhase::Lobby;
        session.local_is_admin = matches!(config.mode, ShooterMode::Host);
    }
    let initial_map_index = session.rules.map_index;
    engine.insert_resource(session);
    engine.insert_resource(ShooterMapState::default());
    if !config.show_main_menu {
        match config.mode {
            ShooterMode::Offline => { let _ = activate_game_map(engine, initial_map_index); }
            ShooterMode::Host | ShooterMode::Join => activate_lobby_map(engine),
        }
    }
}


pub(crate) fn setup_custom_post_processes(engine: &mut Engine, config: &ShooterConfig) {
    let vignette_path = resolve_simple_shooter_asset_path("post_vignette.wgsl");
    let damage_vignette_path = resolve_simple_shooter_asset_path("damage_vignette.wgsl");
    engine.insert_resource(CustomPostProcessStack {
        passes: vec![
            CustomPostProcessPass {
                pass_id: "simple_shooter/vignette".to_string(),
                asset_path: Some(vignette_path.to_string_lossy().into_owned()),
                wgsl_source: None,
                // x = vignette strength, y = inner radius, z = outer radius, w = reserved
                params: vec![0.55, 0.28, 0.92, 0.0],
                order: 100,
                enabled: config.post_vignette || config.show_main_menu,
                input: PostProcessInput::SceneColor,
            },
            CustomPostProcessPass {
                pass_id: "simple_shooter/damage_vignette".to_string(),
                asset_path: Some(damage_vignette_path.to_string_lossy().into_owned()),
                wgsl_source: None,
                // x = health-driven strength, y = inner radius, z = outer radius
                params: vec![0.0, 0.30, 0.78, 0.0],
                order: 110,
                enabled: false,
                input: PostProcessInput::SceneColor,
            },
        ],
    });
}

#[cfg(feature = "gltf")]
pub(crate) fn spawn_optional_car_scene(engine: &mut Engine, config: &ShooterConfig) {
    if !config.load_demo_gltf {
        return;
    }
    if let Err(err) = try_spawn_optional_car_scene(engine) {
        eprintln!("failed to load simple_shooter GLB asset: {err:#}");
    }
}

#[cfg(not(feature = "gltf"))]
pub(crate) fn spawn_optional_car_scene(_engine: &mut Engine, _config: &ShooterConfig) {}

#[cfg(feature = "gltf")]
pub(crate) fn try_spawn_optional_car_scene(engine: &mut Engine) -> anyhow::Result<()> {
    let model_path = resolve_simple_shooter_asset_path("car_scene.glb");
    let root = vetrace_render::load_gltf_actor(engine, model_path.as_path())?;

    // Keep the imported scene game-side and optional: the loader only creates
    // render ECS entities. The shooter chooses where the test model appears.
    if let Some(transform) = root.transform_mut(engine) {
        transform.translation = Vec3::new(0.0, 0.0, -6.0);
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::splat(0.05);
    }
    vetrace_core::propagate_global_transforms(engine);

    println!("loaded simple_shooter GLB asset `{}` as root actor {:?}", model_path.display(), root);
    Ok(())
}

pub(crate) fn resolve_simple_shooter_asset_path(file_name: &str) -> std::path::PathBuf {
    let candidates = [
        std::path::PathBuf::from("assets").join(file_name),
        std::path::PathBuf::from("simple_shooter").join("assets").join(file_name),
    ];

    candidates
        .iter()
        .find(|candidate| candidate.exists())
        .cloned()
        .unwrap_or_else(|| std::path::PathBuf::from("assets").join(file_name))
}


pub(crate) fn spawn_optional_scene_map(engine: &mut Engine, config: &ShooterConfig) {
    let explicit = config.map_json_path.as_ref().map(std::path::PathBuf::from);
    let default_scene = resolve_simple_shooter_asset_path("playground_map.scene.json");
    let default_legacy = resolve_simple_shooter_asset_path("playground_map.json");
    let Some(path) = explicit
        .or_else(|| default_scene.exists().then_some(default_scene))
        .or_else(|| default_legacy.exists().then_some(default_legacy))
    else { return; };
    match vetrace_scene::load_scene_file(&path) {
        Ok(doc) => {
            let name = doc.name.clone();
            match doc.instantiate_with_assets(engine, path.as_path()) {
                Ok((instance, textures)) => {
                    if textures.missing > 0 {
                        eprintln!(
                            "scene map `{name}` loaded, but {} texture file(s) could not be resolved from {}",
                            textures.missing,
                            path.display(),
                        );
                    }
                    println!(
                        "loaded scene map `{name}` from {} ({} objects, {} texture(s) loaded, {} reused)",
                        path.display(),
                        instance.actors.len(),
                        textures.loaded,
                        textures.reused,
                    );
                }
                Err(err) => eprintln!("failed to spawn scene map {}: {err:#}", path.display()),
            }
        }
        Err(err) => {
            eprintln!("failed to load scene map {}: {err:#}", path.display());
        }
    }
}

pub(crate) fn handle_baked_lighting_debug_toggle(engine: &mut Engine) {
    let toggled = engine
        .get_resource::<InputState>()
        .map(|input| input.was_key_pressed("B"))
        .unwrap_or(false);
    if !toggled { return; }
    let mode = cycle_baked_lighting_debug_mode(engine);
    let marker_count = engine.actors_with::<BakedLightProbeDebugMarker>().len();
    println!(
        "baked lighting debug: {mode:?} (B cycles Off/Lightmap/LightmapUv/Probes; {marker_count} probe marker(s))"
    );
}

pub(crate) fn handle_game_window_policy(engine: &mut Engine, _editor_enabled: bool) {
    let should_stop = engine
        .get_resource::<InputState>()
        .map(|input| input.quit_requested())
        .unwrap_or(false);

    if should_stop {
        engine.stop();
    }
}

#[cfg(feature = "editor")]
pub(crate) fn handle_editor_toggle(engine: &mut Engine, runtime: &mut ShooterRuntime) {
    let toggled = engine
        .get_resource::<InputState>()
        .map(|input| input.was_key_pressed("F10"))
        .unwrap_or(false);
    if toggled {
        runtime.editor_enabled = !runtime.editor_enabled;
        println!(
            "editor mode: {}",
            if runtime.editor_enabled { "enabled" } else { "disabled" }
        );
    }

    if let Some(config) = engine.get_resource_mut::<vetrace_editor::EditorConfig>() {
        config.enabled = runtime.editor_enabled;
        config.unlock_cursor = runtime.editor_enabled;
        config.draw_selection_outline = runtime.editor_enabled;
    }

    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = !runtime.editor_enabled;
        settings.cursor_visible = runtime.editor_enabled;
        settings.draw_bounds = runtime.editor_enabled;
    }
}

#[cfg(not(feature = "editor"))]
pub(crate) fn handle_editor_toggle(engine: &mut Engine, runtime: &mut ShooterRuntime) {
    let toggled = engine
        .get_resource::<InputState>()
        .map(|input| input.was_key_pressed("F10"))
        .unwrap_or(false);
    if toggled || runtime.editor_enabled {
        runtime.editor_enabled = false;
        println!("editor mode requires building simple_shooter with --features editor");
    }
    if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
        settings.cursor_grab = true;
        settings.cursor_visible = false;
    }
}
