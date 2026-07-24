use std::collections::BTreeSet;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use vetrace_build::{
    mount_vpak, write_player_template_metadata, BuildError, PlayerTemplateMetadata,
    PlayerTemplateTarget, VPAK_FORMAT_VERSION, PLAYER_TEMPLATE_METADATA_FORMAT_VERSION,
};
use vetrace_project::{ProjectError, ProjectPath, ValidationReport, VetraceProject};
use vetrace_runtime::{LuaProjectHotReloadPlugin, RuntimeError, RuntimeMode, VetraceRuntime};
use vetrace_scripting_lua::{LuaDebuggerCommand, LuaDebuggerEvent, LuaScriptingState};

use crate::{PlayerArgs, PlayerError};

pub fn run(
    args: PlayerArgs,
    output: &mut dyn Write,
    diagnostics: &mut dyn Write,
) -> Result<(), PlayerError> {
    if let Some(directory) = &args.working_directory {
        env::set_current_dir(directory).map_err(|source| PlayerError::WorkingDirectory {
            path: directory.clone(),
            source,
        })?;
    }

    if let Some(player_binary) = args.write_template_metadata.as_ref() {
        let metadata = compiled_player_template_metadata()?;
        let sidecar = write_player_template_metadata(player_binary, &metadata)?;
        writeln!(output, "Wrote player-template metadata: {}", sidecar.display())
            .map_err(output_error)?;
        return Ok(());
    }

    let package_path = args.package.clone().or_else(|| {
        args.project.is_none().then(automatic_sidecar_package).flatten()
    });
    let package_mount = if let Some(package) = package_path.as_ref() {
        Some(mount_vpak(package)?)
    } else {
        None
    };
    let mut project = if let Some(mount) = package_mount.as_ref() {
        mount.project().clone()
    } else {
        VetraceProject::load_unchecked(args.project_path())?
    };

    if let Some(main_scene) = args.main_scene.as_deref() {
        project.manifest_mut().runtime.main_scene = ProjectPath::new(main_scene)?;
    }
    if let Some(fixed_dt) = args.fixed_dt {
        if !fixed_dt.is_finite() || fixed_dt <= 0.0 {
            return Err(PlayerError::InvalidFixedDelta(fixed_dt));
        }
        project.manifest_mut().physics.fixed_timestep = fixed_dt;
    }
    validate_compiled_player(&project, args.headless)?;
    let report = project.validate_files();

    if args.print_project_info {
        write_project_info(&project, output)?;
        if let Some(path) = package_path.as_ref() {
            writeln!(output, "Package: {}", path.display()).map_err(output_error)?;
        }
    }

    if !report.is_valid() {
        return Err(PlayerError::Project(ProjectError::Validation(report)));
    }

    write_warnings(&report, diagnostics)?;

    if args.validate_only {
        writeln!(
            output,
            "Project validation passed: 0 error(s), {} warning(s).",
            report.warning_count()
        )
        .map_err(output_error)?;
        return Ok(());
    }

    if args.print_project_info {
        return Ok(());
    }

    let fixed_dt = project.manifest().physics.fixed_timestep;
    let mode = if args.headless {
        RuntimeMode::HeadlessServer
    } else {
        RuntimeMode::StandaloneGame
    };

    if let Some(path) = package_path.as_ref() {
        writeln!(
            output,
            "Launching packaged '{}' {} from '{}' ({mode:?})",
            project.manifest().project.name,
            project.manifest().project.version,
            path.display(),
        )
        .map_err(output_error)?;
    } else {
        writeln!(
            output,
            "Launching '{}' {} from '{}' ({mode:?})",
            project.manifest().project.name,
            project.manifest().project.version,
            project.root().display(),
        )
        .map_err(output_error)?;
    }

    if args.debug_stdio && !project.manifest().features.scripting {
        return Err(PlayerError::RuntimeSetup(RuntimeError::Plugin(
            "Lua debugger requested, but Lua scripting is disabled in project features".to_owned(),
        )));
    }

    let enable_hot_reload = package_path.is_none()
        && project.manifest().features.scripting
        && project.manifest().scripting.hot_reload;
    let hot_reload_project = project.clone();
    let mut builder = VetraceRuntime::builder(project).mode(mode);
    if enable_hot_reload {
        builder = builder.add_plugin(LuaProjectHotReloadPlugin::new(hot_reload_project));
    }
    let mut runtime = builder.build().map_err(PlayerError::RuntimeSetup)?;
    if args.debug_stdio {
        // Install and configure the hook before RuntimeApp initializes Lua
        // autoloads, scenes, and `ready` callbacks. LuaScriptingPlugin preserves
        // this pre-created state during plugin initialization.
        if !runtime.engine().contains_resource::<LuaScriptingState>() {
            runtime.engine_mut().insert_resource(LuaScriptingState::new());
        }
        start_debug_stdio(&mut runtime).map_err(PlayerError::RuntimeSetup)?;
        runtime.start().map_err(PlayerError::RuntimeSetup)?;
    }

    runtime
        .run_until_stopped(args.max_frames, fixed_dt)
        .map_err(PlayerError::RuntimeExecution)?;
    if args.max_frames.is_some() {
        runtime.stop().map_err(PlayerError::RuntimeExecution)?;
    }

    writeln!(output, "Vetrace Player stopped after {} frame(s).", runtime.frame_count())
        .map_err(output_error)?;
    drop(package_mount);
    Ok(())
}

const DEBUG_COMMAND_PREFIX: &str = "VETRACE_DEBUG_COMMAND\t";
const DEBUG_EVENT_PREFIX: &str = "VETRACE_DEBUG_EVENT\t";

fn start_debug_stdio(runtime: &mut VetraceRuntime) -> Result<(), RuntimeError> {
    let state = runtime
        .engine_mut()
        .get_resource_mut::<LuaScriptingState>()
        .ok_or_else(|| RuntimeError::Plugin(
            "Lua debugger requested, but Lua scripting is not enabled for this project".to_owned(),
        ))?;
    let handle = state
        .enable_debugger()
        .map_err(|error| RuntimeError::Plugin(format!("failed to install Lua debugger: {error}")))?;
    let (controller, events) = handle.into_parts();

    // Start event delivery first so Studio sees `Ready`, then wait for the
    // complete persisted configuration before any project script executes.
    std::thread::spawn(move || {
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        while let Ok(event) = events.recv() {
            let Ok(payload) = serde_json::to_string::<LuaDebuggerEvent>(&event) else { continue; };
            if writeln!(output, "{DEBUG_EVENT_PREFIX}{payload}").is_err() { break; }
            if output.flush().is_err() { break; }
        }
    });

    let stdin = std::io::stdin();
    let mut input = BufReader::new(stdin);
    let mut configured_breakpoints = false;
    let mut configured_watches = false;
    let mut configured_errors = false;
    while !(configured_breakpoints && configured_watches && configured_errors) {
        let mut line = String::new();
        let bytes = input.read_line(&mut line).map_err(|error| {
            RuntimeError::Plugin(format!("failed to read Lua debugger configuration: {error}"))
        })?;
        if bytes == 0 {
            controller.apply(LuaDebuggerCommand::Continue);
            return Err(RuntimeError::Plugin(
                "Lua debugger input closed before Studio sent its initial configuration".to_owned(),
            ));
        }
        let payload = line.trim_end().strip_prefix(DEBUG_COMMAND_PREFIX).unwrap_or(line.trim_end());
        if payload.trim().is_empty() { continue; }
        match serde_json::from_str::<LuaDebuggerCommand>(payload) {
            Ok(command) => {
                match &command {
                    LuaDebuggerCommand::SetBreakpoints { .. } => configured_breakpoints = true,
                    LuaDebuggerCommand::SetWatches { .. } => configured_watches = true,
                    LuaDebuggerCommand::SetBreakOnError { .. } => configured_errors = true,
                    _ => {}
                }
                controller.apply(command);
            }
            Err(error) => eprintln!("invalid Lua debugger command: {error}"),
        }
    }

    std::thread::spawn(move || {
        for line in input.lines() {
            let Ok(line) = line else { break; };
            let payload = line.strip_prefix(DEBUG_COMMAND_PREFIX).unwrap_or(&line);
            if payload.trim().is_empty() { continue; }
            match serde_json::from_str::<LuaDebuggerCommand>(payload) {
                Ok(command) => controller.apply(command),
                Err(error) => eprintln!("invalid Lua debugger command: {error}"),
            }
        }
        // Never leave the runtime thread parked on a breakpoint when the
        // debugger client disconnects or Studio closes its stdin pipe.
        controller.apply(LuaDebuggerCommand::Continue);
    });
    Ok(())
}

fn compiled_player_template_metadata() -> Result<PlayerTemplateMetadata, PlayerError> {
    let target = PlayerTemplateTarget::current().ok_or_else(|| {
        PlayerError::Package(BuildError::Validation(
            "the current player target is not supported by export metadata".to_owned(),
        ))
    })?;
    let mut features = BTreeSet::new();
    if cfg!(any(feature = "window", feature = "software_window")) {
        features.insert("rendering".to_owned());
    }
    features.insert("physics".to_owned());
    if cfg!(feature = "audio_backend") {
        features.insert("audio".to_owned());
    }
    features.insert("animation".to_owned());
    features.insert("networking".to_owned());
    features.insert("ui".to_owned());
    features.insert("scripting".to_owned());
    Ok(PlayerTemplateMetadata {
        format_version: PLAYER_TEMPLATE_METADATA_FORMAT_VERSION,
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        target,
        vpak_format_version: VPAK_FORMAT_VERSION,
        features,
    })
}

fn validate_compiled_player(project: &VetraceProject, headless: bool) -> Result<(), PlayerError> {
    let manifest = project.manifest();
    let compiled_version = env!("CARGO_PKG_VERSION");
    if manifest.project.engine_version != compiled_version {
        return Err(PlayerError::Package(BuildError::Validation(format!(
            "project engine version '{}' is incompatible with player version '{}'",
            manifest.project.engine_version, compiled_version
        ))));
    }
    let features = &manifest.features;
    if features.rendering && !headless && !cfg!(any(feature = "window", feature = "software_window")) {
        return Err(PlayerError::Package(BuildError::Validation(
            "project requires rendering, but this player has no window renderer".to_owned(),
        )));
    }
    if features.audio && !cfg!(feature = "audio_backend") {
        return Err(PlayerError::Package(BuildError::Validation(
            "project requires audio, but this player was built without the audio backend".to_owned(),
        )));
    }
    Ok(())
}

fn automatic_sidecar_package() -> Option<PathBuf> {
    let executable = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(|| env::current_exe().ok())?;
    let directory = executable.parent()?;
    let stem = executable.file_stem()?.to_string_lossy();
    let named = directory.join(format!("{stem}.vpak"));
    if named.is_file() { return Some(named); }
    let conventional = directory.join("game.vpak");
    if conventional.is_file() { return Some(conventional); }
    let packages = std::fs::read_dir(directory).ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("vpak")))
        .take(2)
        .collect::<Vec<_>>();
    (packages.len() == 1).then(|| packages[0].clone())
}

pub fn write_project_info(
    project: &VetraceProject,
    output: &mut dyn Write,
) -> Result<(), PlayerError> {
    let manifest = project.manifest();
    let features = &manifest.features;
    writeln!(output, "Project: {}", manifest.project.name).map_err(output_error)?;
    writeln!(output, "Project ID: {}", manifest.project.id).map_err(output_error)?;
    writeln!(output, "Version: {}", manifest.project.version).map_err(output_error)?;
    writeln!(output, "Engine version: {}", manifest.project.engine_version).map_err(output_error)?;
    writeln!(output, "Format version: {}", manifest.format_version).map_err(output_error)?;
    writeln!(output, "Root: {}", project.root().display()).map_err(output_error)?;
    writeln!(output, "Main scene: {}", manifest.runtime.main_scene).map_err(output_error)?;
    writeln!(output, "Window: {}x{}", manifest.application.width, manifest.application.height)
        .map_err(output_error)?;
    writeln!(output, "Render backend: {:?}", manifest.rendering.backend).map_err(output_error)?;
    writeln!(output, "Fixed timestep: {}", manifest.physics.fixed_timestep).map_err(output_error)?;
    writeln!(
        output,
        "Features: rendering={}, physics={}, audio={}, animation={}, networking={}, ui={}, scripting={}",
        features.rendering,
        features.physics,
        features.audio,
        features.animation,
        features.networking,
        features.ui,
        features.scripting,
    )
    .map_err(output_error)?;
    Ok(())
}

pub fn write_error_diagnostic(
    error: &PlayerError,
    diagnostics: &mut dyn Write,
) -> std::io::Result<()> {
    writeln!(diagnostics, "error: {error}")?;
    match error {
        PlayerError::Project(ProjectError::Validation(report)) => {
            write_report(report, diagnostics)?;
        }
        PlayerError::RuntimeSetup(RuntimeError::Project(ProjectError::Validation(report)))
        | PlayerError::RuntimeExecution(RuntimeError::Project(ProjectError::Validation(report))) => {
            write_report(report, diagnostics)?;
        }
        _ => {}
    }
    Ok(())
}

fn write_warnings(report: &ValidationReport, diagnostics: &mut dyn Write) -> Result<(), PlayerError> {
    for issue in report.warnings() {
        writeln!(diagnostics, "warning: {issue}").map_err(output_error)?;
    }
    Ok(())
}

fn write_report(report: &ValidationReport, diagnostics: &mut dyn Write) -> std::io::Result<()> {
    for issue in report.issues() {
        writeln!(diagnostics, "  {issue}")?;
    }
    Ok(())
}

fn output_error(source: std::io::Error) -> PlayerError {
    PlayerError::RuntimeExecution(RuntimeError::Plugin(format!(
        "failed to write player output: {source}"
    )))
}
