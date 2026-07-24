use super::*;

use vetrace_scripting_lua::{LuaModCommand, LuaModInfo, LuaModManager, LuaModValue};

pub(crate) const MOD_CAP_GAMEPLAY_RULES: &str = "gameplay.rules";
pub(crate) const MOD_CAP_RENDER_POSTPROCESS: &str = "render.postprocess";
pub(crate) const MOD_CAP_UI_STATUS: &str = "ui.status";
pub(crate) const MOD_CMD_MOVEMENT: &str = "movement_multiplier";
pub(crate) const MOD_CMD_JUMP: &str = "jump_multiplier";
pub(crate) const MOD_CMD_GRAVITY: &str = "gravity_scale";
pub(crate) const MOD_CMD_VIGNETTE: &str = "vignette_strength";
pub(crate) const MOD_CMD_STATUS: &str = "status";

pub(crate) fn setup_shooter_modding(engine: &mut Engine, config: &ShooterConfig) {
    let root = resolve_shooter_mods_dir(config);
    let mut manager = LuaModManager::new(&root);
    manager.allow_capability(MOD_CAP_GAMEPLAY_RULES);
    manager.allow_capability(MOD_CAP_RENDER_POSTPROCESS);
    manager.allow_capability(MOD_CAP_UI_STATUS);
    let status = match manager.discover() {
        Ok(count) => {
            manager.enable_saved_and_defaults();
            format!("Discovered {count} Lua mod(s) in {}", root.display())
        }
        Err(err) => format!("Mod discovery failed: {err}"),
    };
    println!("{status}");
    engine.insert_resource(ShooterModEffects::default());
    engine.insert_resource(ShooterModContributions::default());
    engine.insert_resource(ShooterModRuntime { manager, status, watch_elapsed: 0.0 });
}

pub(crate) fn resolve_shooter_mods_dir(config: &ShooterConfig) -> std::path::PathBuf {
    if let Some(path) = &config.mods_dir { return std::path::PathBuf::from(path); }
    let candidates = [
        std::path::PathBuf::from("mods"),
        std::path::PathBuf::from("simple_shooter").join("mods"),
    ];
    candidates.iter().find(|path| path.exists()).cloned().unwrap_or_else(|| candidates[0].clone())
}

pub(crate) fn update_shooter_modding(engine: &mut Engine, time: f32, dt: f32, mode: ShooterMode) {
    let player_count = engine.actors_with::<ShooterPlayer>().len() as f64;
    let (commands, logs) = {
        let Some(runtime) = engine.get_resource_mut::<ShooterModRuntime>() else { return; };
        runtime.watch_elapsed += dt.max(0.0);
        if runtime.watch_elapsed >= 0.5 {
            runtime.watch_elapsed = 0.0;
            for (id, result) in runtime.manager.reload_changed() {
                match result {
                    Ok(()) => println!("Lua mod: [{id}] hot reloaded"),
                    Err(err) => eprintln!("Lua mod: [{id}] hot reload failed: {err}"),
                }
            }
        }
        runtime.manager.set_context_number("time", time as f64);
        runtime.manager.set_context_number("player_count", player_count);
        runtime.manager.set_context_bool("gameplay_active", player_count > 0.0);
        runtime.manager.update(dt);
        (runtime.manager.take_commands(), runtime.manager.take_logs())
    };
    for log in logs { println!("Lua mod: {log}"); }
    for command in commands {
        // Joining clients consume host-authoritative gameplay effects from the
        // network. Local Lua may still log and update UI status, but cannot
        // silently change prediction rules and desynchronize simulation.
        let local_only = command.name == MOD_CMD_STATUS;
        if mode != ShooterMode::Join || local_only { apply_shooter_mod_command(engine, command); }
    }
    apply_shooter_mod_effects(engine);
}

pub(crate) fn shooter_mod_fingerprint(engine: &Engine) -> u64 {
    engine.get_resource::<ShooterModRuntime>().map(|runtime| runtime.manager.active_fingerprint()).unwrap_or(0xcbf29ce484222325)
}

pub(crate) fn shooter_mod_settings(engine: &Engine) -> ShooterModSettings {
    let effects = engine.get_resource::<ShooterModEffects>().copied().unwrap_or_default();
    ShooterModSettings {
        movement_multiplier: effects.movement_multiplier,
        jump_multiplier: effects.jump_multiplier,
        gravity_scale: effects.gravity_scale,
        vignette_strength: effects.vignette_strength,
    }
}

pub(crate) fn apply_authoritative_mod_settings(engine: &mut Engine, settings: ShooterModSettings) {
    engine.insert_resource(ShooterModEffects {
        movement_multiplier: settings.movement_multiplier.clamp(MIN_MOVE_MULTIPLIER, MAX_MOVE_MULTIPLIER),
        jump_multiplier: settings.jump_multiplier.clamp(MIN_JUMP_MULTIPLIER, MAX_JUMP_MULTIPLIER),
        gravity_scale: settings.gravity_scale.clamp(MIN_GRAVITY_SCALE, MAX_GRAVITY_SCALE),
        vignette_strength: settings.vignette_strength.map(|value| value.clamp(0.0, 1.0)),
    });
}

pub(crate) fn apply_shooter_mod_command(engine: &mut Engine, command: LuaModCommand) {
    let mut status = None;
    let required_capability = match command.name.as_str() {
        MOD_CMD_MOVEMENT | MOD_CMD_JUMP | MOD_CMD_GRAVITY => Some(MOD_CAP_GAMEPLAY_RULES),
        MOD_CMD_VIGNETTE => Some(MOD_CAP_RENDER_POSTPROCESS),
        MOD_CMD_STATUS => Some(MOD_CAP_UI_STATUS),
        _ => None,
    };
    if let Some(capability) = required_capability {
        let allowed = shooter_mod_infos(engine).into_iter()
            .find(|info| info.manifest.id == command.mod_id)
            .map(|info| info.manifest.capabilities.iter().any(|declared| declared == capability))
            .unwrap_or(false);
        if !allowed {
            let message = format!("Mod `{}` attempted `{}` without capability `{capability}`", command.mod_id, command.name);
            eprintln!("Lua mod: {message}");
            if let Some(runtime) = engine.get_resource_mut::<ShooterModRuntime>() { runtime.status = message; }
            return;
        }
    }
    if let Some(contributions) = engine.get_resource_mut::<ShooterModContributions>() {
        let effects = contributions.by_mod.entry(command.mod_id.clone()).or_default();
        match (command.name.as_str(), command.value) {
            (MOD_CMD_MOVEMENT, LuaModValue::Number(value)) => {
                effects.movement_multiplier = (value as f32).clamp(MIN_MOVE_MULTIPLIER, MAX_MOVE_MULTIPLIER);
            }
            (MOD_CMD_JUMP, LuaModValue::Number(value)) => {
                effects.jump_multiplier = (value as f32).clamp(MIN_JUMP_MULTIPLIER, MAX_JUMP_MULTIPLIER);
            }
            (MOD_CMD_GRAVITY, LuaModValue::Number(value)) => {
                effects.gravity_scale = (value as f32).clamp(MIN_GRAVITY_SCALE, MAX_GRAVITY_SCALE);
            }
            (MOD_CMD_VIGNETTE, LuaModValue::Number(value)) => {
                effects.vignette_strength = Some((value as f32).clamp(0.0, 1.0));
            }
            (MOD_CMD_VIGNETTE, LuaModValue::Boolean(false)) => effects.vignette_strength = None,
            (MOD_CMD_STATUS, LuaModValue::Text(message)) => status = Some(message),
            (name, _) => {
                status = Some(format!("Mod `{}` emitted unsupported command `{name}`", command.mod_id));
            }
        }
    }
    recompute_shooter_mod_effects(engine);
    if let Some(status) = status {
        println!("Lua mod: {status}");
        if let Some(runtime) = engine.get_resource_mut::<ShooterModRuntime>() { runtime.status = status; }
    }
}

pub(crate) fn recompute_shooter_mod_effects(engine: &mut Engine) {
    let combined = engine.get_resource::<ShooterModContributions>().map(|contributions| {
        let mut combined = ShooterModEffects::default();
        for effects in contributions.by_mod.values() {
            combined.movement_multiplier *= effects.movement_multiplier;
            combined.jump_multiplier *= effects.jump_multiplier;
            combined.gravity_scale *= effects.gravity_scale;
            if effects.vignette_strength.is_some() { combined.vignette_strength = effects.vignette_strength; }
        }
        combined.movement_multiplier = combined.movement_multiplier.clamp(MIN_MOVE_MULTIPLIER, MAX_MOVE_MULTIPLIER);
        combined.jump_multiplier = combined.jump_multiplier.clamp(MIN_JUMP_MULTIPLIER, MAX_JUMP_MULTIPLIER);
        combined.gravity_scale = combined.gravity_scale.clamp(MIN_GRAVITY_SCALE, MAX_GRAVITY_SCALE);
        combined
    }).unwrap_or_default();
    engine.insert_resource(combined);
}

pub(crate) fn apply_shooter_mod_effects(engine: &mut Engine) {
    let effects = engine.get_resource::<ShooterModEffects>().copied().unwrap_or_default();
    if let Some(physics) = engine.get_resource_mut::<vetrace_physics::PhysicsState>() {
        physics.gravity.y = -9.81 * effects.gravity_scale;
    }
    if let Some(strength) = effects.vignette_strength {
        if let Some(stack) = engine.get_resource_mut::<CustomPostProcessStack>() {
            if let Some(pass) = stack.passes.iter_mut().find(|pass| pass.pass_id == "simple_shooter/vignette") {
                if pass.params.len() < 4 { pass.params.resize(4, 0.0); }
                pass.params[0] = strength;
                pass.enabled = strength > 0.0;
            }
        }
    }
}

pub(crate) fn shooter_mod_infos(engine: &Engine) -> Vec<LuaModInfo> {
    engine.get_resource::<ShooterModRuntime>().map(|runtime| runtime.manager.infos()).unwrap_or_default()
}

pub(crate) fn shooter_mod_count(engine: &Engine) -> usize { shooter_mod_infos(engine).len() }

pub(crate) fn selected_shooter_mod(engine: &Engine, selected: usize) -> Option<LuaModInfo> {
    let infos = shooter_mod_infos(engine);
    (!infos.is_empty()).then(|| infos[selected % infos.len()].clone())
}

pub(crate) fn toggle_selected_shooter_mod(engine: &mut Engine, selected: usize) -> String {
    let Some(info) = selected_shooter_mod(engine, selected) else { return "No Lua mods found".to_string(); };
    let result = engine.get_resource_mut::<ShooterModRuntime>()
        .map(|runtime| runtime.manager.toggle(&info.manifest.id))
        .unwrap_or_else(|| Err("Lua mod runtime is unavailable".to_string()));
    match result {
        Ok(enabled) => format!("{} {}", info.manifest.name, if enabled { "enabled" } else { "disabled" }),
        Err(err) => format!("{} failed: {err}", info.manifest.name),
    }
}

pub(crate) fn reload_selected_shooter_mod(engine: &mut Engine, selected: usize) -> String {
    let Some(info) = selected_shooter_mod(engine, selected) else { return "No Lua mods found".to_string(); };
    let result = engine.get_resource_mut::<ShooterModRuntime>()
        .map(|runtime| runtime.manager.reload(&info.manifest.id))
        .unwrap_or_else(|| Err("Lua mod runtime is unavailable".to_string()));
    match result {
        Ok(()) => format!("{} reloaded", info.manifest.name),
        Err(err) => format!("{} reload failed: {err}", info.manifest.name),
    }
}

pub(crate) fn shooter_mod_page_text(engine: &Engine, selected: usize) -> String {
    let infos = shooter_mod_infos(engine);
    if infos.is_empty() {
        let root = engine.get_resource::<ShooterModRuntime>()
            .map(|runtime| runtime.manager.root().display().to_string())
            .unwrap_or_else(|| "mods".to_string());
        return format!("NO MODS FOUND\n\nAdd <folder>/mod.json and main.lua under:\n{root}\n\nThen restart the game.");
    }
    let info = &infos[selected % infos.len()];
    let state = if info.enabled { "ENABLED" } else if info.last_error.is_some() { "ERROR" } else { "DISABLED" };
    let error = info.last_error.as_ref().map(|error| format!("\n\nERROR\n{error}")).unwrap_or_default();
    format!(
        "{}  [{}/{}]\n{} v{}  •  {}\n\n{}{}",
        state,
        selected % infos.len() + 1,
        infos.len(),
        info.manifest.name,
        info.manifest.version,
        if info.manifest.author.is_empty() { "Unknown author" } else { &info.manifest.author },
        info.manifest.description,
        error,
    )
}
