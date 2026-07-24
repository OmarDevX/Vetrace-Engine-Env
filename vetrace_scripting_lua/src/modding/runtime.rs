use super::*;

#[derive(Clone, Copy)]
pub(super) enum Lifecycle { Enable, Disable }

pub(super) fn call_lifecycle(loaded: Option<&LoadedLuaMod>, lifecycle: Lifecycle, api: LuaModApi) -> Result<(), String> {
    let Some(loaded) = loaded else { return Ok(()); };
    let key = match lifecycle { Lifecycle::Enable => loaded.on_enable.as_ref(), Lifecycle::Disable => loaded.on_disable.as_ref() };
    let Some(key) = key else { return Ok(()); };
    let function = loaded.lua.registry_value::<Function>(key).map_err(|err| err.to_string())?;
    loaded.budget.store(loaded.instruction_limit, Ordering::Relaxed);
    function.call::<()>(api).map_err(|err| err.to_string())
}

pub(super) fn call_update(loaded: Option<&LoadedLuaMod>, api: LuaModApi, dt: f32) -> Result<(), String> {
    let Some(loaded) = loaded else { return Ok(()); };
    let Some(key) = loaded.update.as_ref() else { return Ok(()); };
    let function = loaded.lua.registry_value::<Function>(key).map_err(|err| err.to_string())?;
    loaded.budget.store(loaded.instruction_limit, Ordering::Relaxed);
    function.call::<()>((api, dt)).map_err(|err| err.to_string())
}

pub(super) fn load_mod(manifest: &LuaModManifest, directory: &Path, limits: LuaModLimits) -> Result<LoadedLuaMod, String> {
    let entry = safe_entry_path(directory, &manifest.entry)?;
    let source_len = std::fs::metadata(&entry).map_err(|err| format!("inspect {}: {err}", entry.display()))?.len();
    if source_len > limits.max_source_bytes { return Err(format!("mod source exceeds {} byte limit", limits.max_source_bytes)); }
    let source = std::fs::read_to_string(&entry).map_err(|err| format!("read {}: {err}", entry.display()))?;
    let lua = Lua::new();
    lua.set_memory_limit(limits.memory_bytes).map_err(|err| format!("set Lua memory limit: {err}"))?;
    let budget = Arc::new(AtomicU64::new(limits.instructions_per_callback));
    let hook_budget = budget.clone();
    let interval = limits.hook_interval.max(1) as u64;
    lua.set_hook(HookTriggers::new().every_nth_instruction(limits.hook_interval.max(1)), move |_, _| {
        let remaining = hook_budget.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| value.checked_sub(interval));
        if remaining.is_err() { return Err(mlua::Error::RuntimeError("Lua mod instruction budget exceeded".to_string())); }
        Ok(VmState::Continue)
    }).map_err(|err| format!("set Lua execution hook: {err}"))?;
    let env = lua.create_table().map_err(|err| err.to_string())?;
    copy_safe_globals(&lua, &env).map_err(|err| err.to_string())?;
    let value = lua.load(&source).set_name(format!("{}:{}", manifest.id, manifest.entry)).set_environment(env.clone()).eval::<Value>().map_err(|err| err.to_string())?;
    let table = match value { Value::Table(table) => table, _ => env };
    let on_enable = registry_function(&lua, &table, "on_enable").map_err(|err| err.to_string())?;
    let on_disable = registry_function(&lua, &table, "on_disable").map_err(|err| err.to_string())?;
    let update = registry_function(&lua, &table, "update").map_err(|err| err.to_string())?;
    Ok(LoadedLuaMod { lua, on_enable, on_disable, update, budget, instruction_limit: limits.instructions_per_callback })
}

fn registry_function(lua: &Lua, table: &Table, name: &str) -> mlua::Result<Option<RegistryKey>> {
    match table.get::<Option<Function>>(name)? { Some(function) => Ok(Some(lua.create_registry_value(function)?)), None => Ok(None) }
}

fn copy_safe_globals(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let globals = lua.globals();
    env.set("_G", env.clone())?;
    for name in ["assert", "error", "ipairs", "next", "pairs", "pcall", "select", "tonumber", "tostring", "type", "xpcall"] { if let Ok(value) = globals.get::<Value>(name) { env.set(name, value)?; } }
    for name in ["math", "string", "table", "utf8", "coroutine"] { if let Ok(value) = globals.get::<Table>(name) { env.set(name, value)?; } }
    Ok(())
}

pub(super) fn validate_manifest(manifest: &LuaModManifest) -> Result<(), String> {
    if manifest.id.is_empty() || !manifest.id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') { return Err("mod id must contain only ASCII letters, numbers, '-' or '_'".to_string()); }
    if manifest.name.trim().is_empty() { return Err(format!("mod `{}` has an empty name", manifest.id)); }
    safe_entry_path(Path::new("."), &manifest.entry).map(|_| ())
}

pub(super) fn safe_entry_path(directory: &Path, entry: &str) -> Result<PathBuf, String> {
    let relative = Path::new(entry);
    if relative.is_absolute() || relative.components().any(|part| !matches!(part, Component::Normal(_))) { return Err(format!("unsafe mod entry path `{entry}`")); }
    if relative.extension().and_then(|ext| ext.to_str()) != Some("lua") { return Err(format!("mod entry `{entry}` must be a .lua file")); }
    Ok(directory.join(relative))
}

pub(super) fn read_enabled_state_file(path: &Path) -> Result<BTreeMap<String, bool>, String> {
    let source = match std::fs::read_to_string(path) { Ok(source) => source, Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()), Err(err) => return Err(format!("read {}: {err}", path.display())) };
    serde_json::from_str(&source).map_err(|err| format!("parse {}: {err}", path.display()))
}

pub(super) fn fallback_state_path(root: &Path) -> PathBuf {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in root.to_string_lossy().bytes() { hash ^= byte as u64; hash = hash.wrapping_mul(0x100000001b3); }
    std::path::PathBuf::from(".vetrace").join("mod_state").join(format!("{hash:016x}.json"))
}

pub(super) fn hash_file(path: &Path) -> Result<u64, String> {
    let bytes = std::fs::read(path).map_err(|err| format!("read {} for hashing: {err}", path.display()))?;
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes { hash ^= byte as u64; hash = hash.wrapping_mul(0x100000001b3); }
    Ok(hash)
}
