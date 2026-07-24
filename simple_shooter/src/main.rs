mod components;
mod net;
mod replication;
mod app;

use std::error::Error;

use components::{shooter_initial_render_settings, ShooterConfig, ShooterGraphicsProfile, ShooterMode, ShooterProfileUiMode};
use vetrace_render::AdapterPreference;
use app::SimpleShooterApp;
use vetrace_core::AppBuilder;
use vetrace_net::NetPlugin;
use vetrace_physics::RapierPhysicsPlugin;
use vetrace_pathfinding::PathfindingPlugin;
use vetrace_render::RenderPlugin;
use vetrace_ui::UiPlugin;
#[cfg(feature = "gltf")]
use vetrace_animation::AnimationPlugin;
#[cfg(feature = "audio")]
use vetrace_audio::AudioPlugin;
#[cfg(feature = "profiler")]
use vetrace_profiler::{ProfilerConfig, ProfilerPlugin, ProfilerUiMode};
#[cfg(feature = "editor")]
use vetrace_editor::{EditorConfig, EditorPlugin};

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_args();
    print_banner(&config);

    let mut builder = AppBuilder::new()
        .insert_resource(shooter_initial_render_settings(&config));

    #[cfg(feature = "profiler")]
    if config.profile_enabled {
        builder = builder.add_plugin(ProfilerPlugin::new(ProfilerConfig {
            print_interval: std::time::Duration::from_secs(2),
            top_timing_count: 18,
            ui_mode: match config.profile_ui_mode {
                ShooterProfileUiMode::Detached => ProfilerUiMode::Detached,
                ShooterProfileUiMode::Overlay => ProfilerUiMode::Overlay,
                ShooterProfileUiMode::Both => ProfilerUiMode::Both,
            },
            ..ProfilerConfig::default()
        }));
    }

    #[cfg(not(feature = "profiler"))]
    if config.profile_enabled {
        eprintln!("--profile requested, but simple_shooter was not built with --features profiler");
    }

    builder = builder
        // Engine feature crates stay reusable plugins. The shooter itself is the App.
        .add_plugin(RenderPlugin::new())
        .add_plugin(UiPlugin::new())
        .add_plugin(NetPlugin::new())
        .add_plugin(PathfindingPlugin::default())
        .add_plugin(RapierPhysicsPlugin::new());

    #[cfg(feature = "gltf")]
    {
        // GLTF animation playback is runtime-side; rendering only imports clips.
        builder = builder.add_plugin(AnimationPlugin::new());
    }

    #[cfg(feature = "audio")]
    {
        // Audio runs after physics so spatial emitters hear the latest synced transforms.
        builder = builder.add_plugin(AudioPlugin::new());
    }

    #[cfg(feature = "editor")]
    {
        let mut editor_config = EditorConfig::default();
        // Keep normal FPS mode untouched by default. Runtime F10/--editor flips
        // this resource before the editor plugin runs each frame.
        editor_config.enabled = config.editor_enabled;
        editor_config.unlock_cursor = config.editor_enabled;
        editor_config.draw_selection_outline = config.editor_enabled;
        builder = builder.add_plugin(EditorPlugin::with_config(editor_config));
    }

    #[cfg(not(feature = "editor"))]
    if config.editor_enabled {
        eprintln!("--editor requested, but simple_shooter was not built with --features editor");
    }

    let app = SimpleShooterApp::new(config.clone())?;
    match config.max_frames {
        Some(frames) => builder.run_frames(app, frames, 1.0 / 60.0),
        None => builder.run_until_stopped(app, None, 1.0 / 60.0),
    }
}

fn parse_args() -> ShooterConfig {
    let mut config = ShooterConfig::default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut name_from_args = false;
    let mut prompt_for_name = true;
    let mut main_menu_override = None;
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "offline" => config.mode = ShooterMode::Offline,
            "host" => config.mode = ShooterMode::Host,
            "join" => config.mode = ShooterMode::Join,
            "--bind" => {
                if let Some(value) = args.get(i + 1) {
                    config.bind_addr = value.clone();
                    i += 1;
                }
            }
            "--server" => {
                if let Some(value) = args.get(i + 1) {
                    config.server_addr = value.clone();
                    i += 1;
                }
            }
            "--bots" | "--bot-count" => {
                if let Some(value) = args.get(i + 1) {
                    config.bot_count = value.parse().unwrap_or(config.bot_count);
                    i += 1;
                }
            }
            "--max-players" => {
                if let Some(value) = args.get(i + 1) {
                    config.max_players = value.parse().unwrap_or(config.max_players);
                    i += 1;
                }
            }
            "--discovery-port" => {
                if let Some(value) = args.get(i + 1) {
                    config.discovery_port = value.parse().unwrap_or(config.discovery_port);
                    i += 1;
                }
            }
            "--name" => {
                if let Some(value) = args.get(i + 1) {
                    config.player_name = sanitize_cli_player_name(value);
                    name_from_args = true;
                    i += 1;
                }
            }
            "--no-name-prompt" => prompt_for_name = false,
            "--frames" => {
                if let Some(value) = args.get(i + 1) {
                    config.max_frames = value.parse().ok();
                    i += 1;
                }
            }
            "--scripted-input" => config.use_scripted_input = true,
            "--no-scripted-input" => config.use_scripted_input = false,
            "--editor" => config.editor_enabled = true,
            "--no-editor" => config.editor_enabled = false,
            "--profile" => {
                config.profile_enabled = true;
                config.profile_ui_mode = ShooterProfileUiMode::Detached;
            }
            "--profile-detached" => {
                config.profile_enabled = true;
                config.profile_ui_mode = ShooterProfileUiMode::Detached;
            }
            "--profile-overlay" => {
                config.profile_enabled = true;
                config.profile_ui_mode = ShooterProfileUiMode::Overlay;
            }
            "--profile-both" => {
                config.profile_enabled = true;
                config.profile_ui_mode = ShooterProfileUiMode::Both;
            }
            "--no-profile" => config.profile_enabled = false,
            "--low-spec" | "--low" | "--potato" => {
                config.graphics_profile = ShooterGraphicsProfile::LowSpec;
                config.graphics_profile_explicit = true;
            }
            "--balanced" | "--medium" => {
                config.graphics_profile = ShooterGraphicsProfile::Balanced;
                config.graphics_profile_explicit = true;
            }
            "--high-quality" | "--high" => {
                config.graphics_profile = ShooterGraphicsProfile::HighQuality;
                config.graphics_profile_explicit = true;
            }
            "--fog" | "--volumetric-fog" => config.force_fog = Some(true),
            "--no-fog" | "--no-volumetric-fog" => config.force_fog = Some(false),
            "--shadows" => config.force_shadows = Some(true),
            "--no-shadows" | "--disable-shadows" => config.force_shadows = Some(false),
            "--vsync" => config.vsync = Some(true),
            "--no-vsync" | "--low-latency-present" => config.vsync = Some(false),
            "--integrated-gpu" | "--igpu" | "--low-power-gpu" => {
                config.adapter_preference = Some(AdapterPreference::LowPower);
            }
            "--discrete-gpu" | "--dgpu" | "--high-performance-gpu" => {
                config.adapter_preference = Some(AdapterPreference::HighPerformance);
            }
            "--bake-lighting" | "--bake-gi" => config.bake_lighting = true,
            "--post-vignette" | "--vignette" => config.post_vignette = true,
            "--no-post-vignette" | "--no-vignette" => config.post_vignette = false,
            "--main-menu" => main_menu_override = Some(true),
            "--no-main-menu" => main_menu_override = Some(false),
            "--mods-dir" => {
                if let Some(value) = args.get(i + 1) {
                    config.mods_dir = Some(value.clone());
                    i += 1;
                }
            }
            "--no-gltf-demo" | "--no-car-scene" => config.load_demo_gltf = false,
            "--gltf-demo" | "--car-scene" => config.load_demo_gltf = true,
            "--scene-json" | "--map-json" | "--prefab" => {
                if let Some(value) = args.get(i + 1) {
                    config.map_json_path = Some(value.clone());
                    i += 1;
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => eprintln!("unknown argument: {other}"),
        }
        i += 1;
    }

    let interactive_game = config.max_frames.is_none() && !config.use_scripted_input;
    config.show_main_menu = main_menu_override.unwrap_or(interactive_game);
    if !name_from_args {
        config.player_name = sanitize_cli_player_name(&config.player_name);
        config.prompt_player_name_in_ui = prompt_for_name && interactive_game;
    }

    config
}

fn sanitize_cli_player_name(name: &str) -> String {
    let cleaned = name
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(24)
        .collect::<String>();
    if cleaned.is_empty() { "Player".to_string() } else { cleaned }
}

fn print_help() {
    println!(r#"Simple Shooter

Usage:
  cargo run -p simple_shooter
  cargo run -p simple_shooter -- [OPTIONS]

Controls:
  WASD = move
  Mouse = look
  Left mouse = shoot
  Space = jump
  Escape = open or close the pause menu
  F10 = toggle editor mode when built with --features editor

Options:
  --name NAME = skip the in-window name screen and use this player name
  --no-name-prompt = keep the default name without asking
  --profile / --profile-detached = open a separate native vetrace_profiler window and print timings/counters every 2 seconds
  --profile-overlay = draw the profiler inside the game window instead
  --profile-both = draw both detached and overlay profiler UIs
  --low-spec / --balanced / --high-quality = choose graphics cost profile; default is balanced
  --fog / --no-fog = start with volumetric fog on or off
  --shadows / --no-shadows = start with directional shadows on or off
  --vsync / --no-vsync = start with VSync on or off
  --integrated-gpu / --discrete-gpu = choose the preferred GPU
  --bake-lighting = explicitly CPU-bake the active map's lightmaps/probes and save a .vlight file
                     (combine with --no-main-menu to bake the first gameplay map immediately)
  --post-vignette / --no-post-vignette = start with vignette on or off
  --main-menu / --no-main-menu = force or skip the in-window front end
  --mods-dir PATH = load manifest-driven Lua mods from PATH
  --bots N = set requested bot count for offline and bot-enabled hosted games
  --max-players N = cap human players; automatically limited by map spawn capacity
  --discovery-port PORT = UDP port used for LAN server discovery
  --gltf-demo / --no-gltf-demo = load or skip the bundled glTF scene
  --scene-json PATH = import a vetrace_scene JSON map exported by vetrace_map_builder
  --map-json PATH = compatibility alias for --scene-json
"#);
}

fn print_banner(config: &ShooterConfig) {
    println!("Simple Shooter starting in {:?} mode", config.mode);
    match config.mode {
        ShooterMode::Offline => println!("offline bot arena; no network socket required"),
        ShooterMode::Host => println!("hosting authoritative server on {}", config.bind_addr),
        ShooterMode::Join => println!("joining authoritative server at {}", config.server_addr),
    }
    println!("graphics profile: {:?}", config.graphics_profile);
    if let Some(force_shadows) = config.force_shadows {
        println!("directional shadows override: {}", if force_shadows { "on" } else { "off" });
    }
    #[cfg(feature = "gltf")]
    println!("glTF scene: {}", if config.load_demo_gltf { "on" } else { "off" });
    if let Some(vsync) = config.vsync {
        println!("present mode override: {}", if vsync { "vsync" } else { "low latency/no-vsync" });
    }
    if let Some(adapter_preference) = config.adapter_preference {
        println!("adapter preference override: {:?}", adapter_preference);
    }
    if config.prompt_player_name_in_ui {
        println!("player name: choose in game window");
    } else {
        println!("player name: {}", config.player_name);
    }
    if config.bake_lighting {
        println!("baked lighting: explicit bake mode (normal runs only load existing .vlight files)");
    }
    if config.editor_enabled {
        println!("editor mode: enabled at startup");
    } else {
        println!("editor mode: off (build with --features editor and press F10 or pass --editor)");
    }
    if config.profile_enabled {
        println!("profiler: enabled (vetrace_profiler {:?} UI + console output)", config.profile_ui_mode);
    } else {
        println!("profiler: off (build with --features profiler and pass --profile)");
    }
}
