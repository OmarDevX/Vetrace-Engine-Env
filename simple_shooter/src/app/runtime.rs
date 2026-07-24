use super::*;

/// Game-side application for Simple Shooter.
///
/// This is intentionally an `App`, not a `Plugin`: the shooter owns the game
/// loop, world setup, player input, damage rules, crosshair policy, and typed
/// shooter payloads. The packet envelope and reusable session mechanics live
/// in `vetrace_net`.
mod network_state;
pub use network_state::*;

pub struct SimpleShooterApp {
    runtime: ShooterRuntime,
}

impl SimpleShooterApp {
    pub fn new(mut config: ShooterConfig) -> Result<Self, Box<dyn Error>> {
        let saved = ShooterGameSettings::load();
        // A profile flag is authoritative for both headless gameplay and
        // explicit baking. Without one, normal gameplay honors the saved UI
        // setting while bake mode keeps its CLI/default profile.
        if !config.bake_lighting && !config.graphics_profile_explicit {
            config.graphics_profile = saved.graphics_profile;
        }
        config.vsync = Some(saved.vsync);
        config.post_vignette = saved.vignette;
        config.force_fog = Some(config.force_fog.unwrap_or(saved.volumetric_fog));
        Ok(Self { runtime: ShooterRuntime::new(config)? })
    }
}

impl App for SimpleShooterApp {
    fn setup(&mut self, engine: &mut Engine) {
        let mut settings = ShooterGameSettings::load();
        if let Some(enabled) = self.runtime.config.force_fog {
            settings.volumetric_fog = enabled;
        }
        engine.insert_resource(settings);
        engine.insert_resource(PauseMenuState::default());
        engine.insert_resource(ShooterPresentationConfig::default());
        register_components(engine);
        load_active_weapon(engine);
        setup_render_resources(engine, &self.runtime.config);
        setup_world(engine, &self.runtime.config);
        setup_shooter_modding(engine, &self.runtime.config);
        if self.runtime.config.show_main_menu {
            setup_main_menu(engine, &self.runtime);
        } else {
            setup_join_menu_ui(engine, &self.runtime.config);
        }
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) {
        self.runtime.time += dt;
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.time_seconds = self.runtime.time;
        }
        apply_live_game_settings(engine);
        update_server_directory(engine, &mut self.runtime);
        handle_editor_toggle(engine, &mut self.runtime);
        update_shooter_modding(engine, self.runtime.time, dt, self.runtime.config.mode);
        if update_main_menu(engine, &mut self.runtime, dt) {
            if self.runtime.background_hosting {
                update_host(engine, &mut self.runtime, dt);
                hide_background_gameplay(engine);
            }
            return;
        }
        if update_join_menu(engine, &mut self.runtime) {
            return;
        }
        handle_baked_lighting_debug_toggle(engine);
        update_pause_menu(engine, &mut self.runtime);
        if engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false) {
            return;
        }
        handle_game_window_policy(engine, self.runtime.editor_enabled);
        handle_free_flight_toggle_and_speed(engine, &self.runtime);
        match self.runtime.config.mode {
            ShooterMode::Offline => update_offline(engine, &mut self.runtime, dt),
            ShooterMode::Host => update_host(engine, &mut self.runtime, dt),
            ShooterMode::Join => update_client(engine, &mut self.runtime, dt),
        }
        update_lobby_ui(engine, &mut self.runtime);
        update_kills_leaderboard(engine);
        update_free_flight_controller(engine, &self.runtime, dt);

        run_presentation_stage(engine, &self.runtime, dt);

        // Frame pacing is left to the WGPU window/vsync or an external runtime.
    }

    fn render(&mut self, engine: &mut Engine) {
        // Physics plugins run after App::update and before App::render.  Send
        // host snapshots here, after Rapier has written the final Transform
        // back into the world.  If we send inside update_host(), the snapshot
        // sees the pre-physics yaw-only rotation written by input handling and
        // remote clients lose the physical/body tilt.
        flush_post_physics_host_snapshot(engine, &mut self.runtime);

        // The outline shell is a separate game-owned visual entity, so sync it
        // here as well to keep it locked to the final post-physics player
        // Transform.
        run_post_physics_presentation_stage(engine);
    }
}

impl Drop for SimpleShooterApp {
    fn drop(&mut self) {
        if let Some(server) = self.runtime.server.as_ref() {
            let packet = ShooterMessage::Shutdown {
                reason: "The host closed the server. You were returned safely to the main menu.".to_string(),
            };
            // UDP is intentionally repeated: shutdown must remain best-effort even
            // though the process cannot wait for an acknowledgement while exiting.
            server.net.broadcast_message(packet.clone());
            server.net.broadcast_message(packet.clone());
            server.net.broadcast_message(packet);
        }
    }
}

pub struct ShooterRuntime {
    pub config: ShooterConfig,
    pub time: f32,
    pub local_id: Option<u64>,
    pub local_seed: u64,
    pub server: Option<ServerState>,
    pub client: Option<ClientState>,
    pub offline_initialized: bool,
    pub editor_enabled: bool,
    pub name_ready: bool,
    pub browser: Option<ServerBrowser>,
    pub advertiser: Option<ServerAdvertiser>,
    pub background_hosting: bool,
    pub local_host_participating: bool,
}

impl ShooterRuntime {
    pub fn new(mut config: ShooterConfig) -> Result<Self, Box<dyn Error>> {
        configure_external_maps(&config);
        config.max_players = config.max_players.clamp(1, minimum_map_capacity());
        config.bot_count = config.bot_count.min(MAX_BOT_COUNT);
        let seed = automatic_player_color_seed(stable_hash(&config.player_name));
        let (server, client, local_id) = match config.mode {
            ShooterMode::Offline => (None, None, Some(SERVER_AUTHORITY_ID)),
            ShooterMode::Host => {
                let channel = TypedUdpChannel::<ShooterPacket>::bind(&config.bind_addr)?;
                println!("Simple Shooter hosting on {}", channel.local_addr()?);
                (Some(ServerState::new(channel, config.max_players)), None, Some(SERVER_AUTHORITY_ID))
            }
            ShooterMode::Join => {
                let channel = TypedUdpChannel::<ShooterPacket>::bind("0.0.0.0:0")?;
                let server_addr: SocketAddr = config.server_addr.parse()?;
                println!("Simple Shooter joining {server_addr} from {}", channel.local_addr()?);
                (None, Some(ClientState::new(channel, server_addr, config.player_name.clone(), seed)), None)
            }
        };

        let editor_enabled = config.editor_enabled;
        let name_ready = !config.prompt_player_name_in_ui;
        let browser = ServerBrowser::new(LanDiscoveryConfig::shooter(config.discovery_port)).ok();
        Ok(Self {
            config,
            time: 0.0,
            local_id,
            local_seed: seed,
            server,
            client,
            offline_initialized: false,
            editor_enabled,
            name_ready,
            browser,
            advertiser: None,
            background_hosting: false,
            local_host_participating: true,
        })
    }

    pub fn host_from_menu(&mut self) -> Result<(), Box<dyn Error>> {
        let channel = TypedUdpChannel::<ShooterPacket>::bind(&self.config.bind_addr)?;
        let local = channel.local_addr()?;
        let advertiser = ServerAdvertiser::new(
            format!("{}'s server", self.config.player_name),
            local.port(),
            LanDiscoveryConfig::shooter(self.config.discovery_port),
        )?;
        self.server = Some(ServerState::new(channel, self.config.max_players));
        self.client = None;
        self.local_id = Some(SERVER_AUTHORITY_ID);
        self.config.mode = ShooterMode::Host;
        self.advertiser = Some(advertiser);
        self.background_hosting = false;
        self.local_host_participating = true;
        Ok(())
    }

    pub fn join_from_menu(&mut self, server_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
        let channel = TypedUdpChannel::<ShooterPacket>::bind("0.0.0.0:0")?;
        self.client = Some(ClientState::new(channel, server_addr, self.config.player_name.clone(), self.local_seed));
        self.server = None;
        self.local_id = None;
        self.config.server_addr = server_addr.to_string();
        self.config.mode = ShooterMode::Join;
        self.advertiser = None;
        self.background_hosting = false;
        self.local_host_participating = true;
        Ok(())
    }
}
