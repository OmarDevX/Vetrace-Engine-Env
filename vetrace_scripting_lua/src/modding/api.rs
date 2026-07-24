use super::*;

pub(super) struct LoadedLuaMod {
    pub(super) lua: Lua,
    pub(super) on_enable: Option<RegistryKey>,
    pub(super) on_disable: Option<RegistryKey>,
    pub(super) update: Option<RegistryKey>,
    pub(super) budget: Arc<AtomicU64>,
    pub(super) instruction_limit: u64,
}

pub(super) struct DiscoveredLuaMod {
    pub(super) manifest: LuaModManifest,
    pub(super) directory: PathBuf,
    pub(super) enabled: bool,
    pub(super) loaded: Option<LoadedLuaMod>,
    pub(super) last_error: Option<String>,
    pub(super) source_hash: u64,
}

#[derive(Clone)]
pub(super) struct LuaModApi {
    pub(super) mod_id: String,
    pub(super) context: Arc<Mutex<HashMap<String, LuaModValue>>>,
    pub(super) commands: Arc<Mutex<Vec<LuaModCommand>>>,
    pub(super) logs: Arc<Mutex<Vec<String>>>,
    pub(super) state: Arc<Mutex<HashMap<String, HashMap<String, LuaModValue>>>>,
}

impl LuaModApi {
    fn emit(&self, name: String, value: LuaModValue) {
        if let Ok(mut commands) = self.commands.lock() {
            commands.push(LuaModCommand { mod_id: self.mod_id.clone(), name, value });
        }
    }
}

impl UserData for LuaModApi {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_number", |_, this, key: String| {
            Ok(this.context.lock().ok().and_then(|values| match values.get(&key) {
                Some(LuaModValue::Number(value)) => Some(*value),
                _ => None,
            }))
        });
        methods.add_method("get_bool", |_, this, key: String| {
            Ok(this.context.lock().ok().and_then(|values| match values.get(&key) {
                Some(LuaModValue::Boolean(value)) => Some(*value),
                _ => None,
            }))
        });
        methods.add_method("get_string", |_, this, key: String| {
            Ok(this.context.lock().ok().and_then(|values| match values.get(&key) {
                Some(LuaModValue::Text(value)) => Some(value.clone()),
                _ => None,
            }))
        });
        methods.add_method("emit_number", |_, this, (name, value): (String, f64)| {
            this.emit(name, LuaModValue::Number(value));
            Ok(())
        });
        methods.add_method("emit_bool", |_, this, (name, value): (String, bool)| {
            this.emit(name, LuaModValue::Boolean(value));
            Ok(())
        });
        methods.add_method("emit_string", |_, this, (name, value): (String, String)| {
            this.emit(name, LuaModValue::Text(value));
            Ok(())
        });
        methods.add_method("log", |_, this, message: String| {
            if let Ok(mut logs) = this.logs.lock() {
                logs.push(format!("[{}] {message}", this.mod_id));
            }
            Ok(())
        });
        methods.add_method("state_get_number", |_, this, key: String| {
            Ok(this.state.lock().ok().and_then(|state| state.get(&this.mod_id).and_then(|values| match values.get(&key) { Some(LuaModValue::Number(value)) => Some(*value), _ => None })))
        });
        methods.add_method("state_set_number", |_, this, (key, value): (String, f64)| {
            if let Ok(mut state) = this.state.lock() { state.entry(this.mod_id.clone()).or_default().insert(key, LuaModValue::Number(value)); }
            Ok(())
        });
        methods.add_method("state_get_string", |_, this, key: String| {
            Ok(this.state.lock().ok().and_then(|state| state.get(&this.mod_id).and_then(|values| match values.get(&key) { Some(LuaModValue::Text(value)) => Some(value.clone()), _ => None })))
        });
        methods.add_method("state_set_string", |_, this, (key, value): (String, String)| {
            if let Ok(mut state) = this.state.lock() { state.entry(this.mod_id.clone()).or_default().insert(key, LuaModValue::Text(value)); }
            Ok(())
        });
    }
}
