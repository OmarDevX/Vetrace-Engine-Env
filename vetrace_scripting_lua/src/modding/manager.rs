use super::*;

pub struct LuaModManager {
    root: PathBuf,
    mods: Vec<DiscoveredLuaMod>,
    saved_enabled: BTreeMap<String, bool>,
    context: Arc<Mutex<HashMap<String, LuaModValue>>>,
    commands: Arc<Mutex<Vec<LuaModCommand>>>,
    logs: Arc<Mutex<Vec<String>>>,
    limits: LuaModLimits,
    allowed_capabilities: std::collections::HashSet<String>,
    fallback_state_path: PathBuf,
    mod_state: Arc<Mutex<HashMap<String, HashMap<String, LuaModValue>>>>,
}

impl LuaModManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let fallback_state_path = fallback_state_path(&root);
        let primary_state_path = root.join("enabled_mods.json");
        let saved_enabled = if primary_state_path.exists() {
            read_enabled_state_file(&primary_state_path).unwrap_or_default()
        } else {
            read_enabled_state_file(&fallback_state_path).unwrap_or_default()
        };
        Self {
            root,
            mods: Vec::new(),
            saved_enabled,
            context: Arc::new(Mutex::new(HashMap::new())),
            commands: Arc::new(Mutex::new(Vec::new())),
            logs: Arc::new(Mutex::new(Vec::new())),
            limits: LuaModLimits::default(),
            allowed_capabilities: std::collections::HashSet::new(),
            fallback_state_path,
            mod_state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_limits(mut self, limits: LuaModLimits) -> Self { self.limits = limits; self }
    pub fn allow_capability(&mut self, capability: impl Into<String>) { self.allowed_capabilities.insert(capability.into()); }

    pub fn root(&self) -> &Path { &self.root }

    pub fn discover(&mut self) -> Result<usize, String> {
        std::fs::create_dir_all(&self.root).map_err(|err| format!("create mod directory: {err}"))?;
        let mut discovered = Vec::new();
        let entries = std::fs::read_dir(&self.root).map_err(|err| format!("read mod directory: {err}"))?;
        for entry in entries {
            let entry = entry.map_err(|err| format!("read mod entry: {err}"))?;
            let directory = entry.path();
            if !directory.is_dir() { continue; }
            let manifest_path = directory.join("mod.json");
            if !manifest_path.is_file() { continue; }
            let source = std::fs::read_to_string(&manifest_path)
                .map_err(|err| format!("read {}: {err}", manifest_path.display()))?;
            let manifest: LuaModManifest = serde_json::from_str(&source)
                .map_err(|err| format!("parse {}: {err}", manifest_path.display()))?;
            validate_manifest(&manifest)?;
            if discovered.iter().any(|item: &DiscoveredLuaMod| item.manifest.id == manifest.id) {
                return Err(format!("duplicate mod id `{}`", manifest.id));
            }
            let enabled = self.saved_enabled.get(&manifest.id).copied().unwrap_or(manifest.enabled_by_default);
            let source_hash = hash_file(&safe_entry_path(&directory, &manifest.entry)?)?;
            discovered.push(DiscoveredLuaMod {
                manifest,
                directory,
                enabled: false,
                loaded: None,
                last_error: None,
                source_hash,
            });
            if enabled {
                let id = discovered.last().expect("mod was just pushed").manifest.id.clone();
                self.saved_enabled.insert(id, true);
            }
        }
        discovered.sort_by(|a, b| a.manifest.priority.cmp(&b.manifest.priority).then_with(|| a.manifest.id.cmp(&b.manifest.id)));
        self.mods = discovered;
        Ok(self.mods.len())
    }

    pub fn enable_saved_and_defaults(&mut self) {
        let ids = self.mods.iter()
            .filter(|item| self.saved_enabled.get(&item.manifest.id).copied().unwrap_or(item.manifest.enabled_by_default))
            .map(|item| item.manifest.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            let _ = self.enable(&id);
        }
    }

    pub fn infos(&self) -> Vec<LuaModInfo> {
        self.mods.iter().map(|item| LuaModInfo {
            manifest: item.manifest.clone(),
            enabled: item.enabled,
            loaded: item.loaded.is_some(),
            last_error: item.last_error.clone(),
        }).collect()
    }

    pub fn enable(&mut self, id: &str) -> Result<(), String> {
        self.enable_inner(id, &mut Vec::new())
    }

    fn enable_inner(&mut self, id: &str, stack: &mut Vec<String>) -> Result<(), String> {
        if stack.iter().any(|item| item == id) { return Err(format!("dependency cycle: {} -> {id}", stack.join(" -> "))); }
        let index = self.index_of(id)?;
        if self.mods[index].enabled { return Ok(()); }
        for capability in &self.mods[index].manifest.capabilities {
            if !self.allowed_capabilities.contains(capability) { return Err(format!("mod `{id}` requests unavailable capability `{capability}`")); }
        }
        for conflict in &self.mods[index].manifest.conflicts {
            if self.mods.iter().any(|item| item.enabled && item.manifest.id == *conflict) { return Err(format!("mod `{id}` conflicts with enabled mod `{conflict}`")); }
        }
        if let Some(conflicting) = self.mods.iter().find(|item| item.enabled && item.manifest.conflicts.iter().any(|conflict| conflict == id)) {
            return Err(format!("enabled mod `{}` conflicts with `{id}`", conflicting.manifest.id));
        }
        let dependencies = self.mods[index].manifest.dependencies.clone();
        stack.push(id.to_string());
        for dependency in dependencies {
            let dependency_index = self.index_of(&dependency.id)?;
            if let Some(required) = &dependency.version {
                if &self.mods[dependency_index].manifest.version != required { return Err(format!("mod `{id}` requires {} version {required}", dependency.id)); }
            }
            self.enable_inner(&dependency.id, stack)?;
        }
        stack.pop();
        if self.mods[index].loaded.is_none() {
            match load_mod(&self.mods[index].manifest, &self.mods[index].directory, self.limits) {
                Ok(loaded) => self.mods[index].loaded = Some(loaded),
                Err(err) => {
                    self.mods[index].last_error = Some(err.clone());
                    return Err(err);
                }
            }
        }
        let api = self.api_for(index);
        if let Err(err) = call_lifecycle(self.mods[index].loaded.as_ref(), Lifecycle::Enable, api) {
            self.mods[index].last_error = Some(err.clone());
            return Err(err);
        }
        self.mods[index].enabled = true;
        self.mods[index].last_error = None;
        self.saved_enabled.insert(id.to_string(), true);
        self.save_enabled_state();
        Ok(())
    }

    pub fn disable(&mut self, id: &str) -> Result<(), String> {
        let index = self.index_of(id)?;
        if !self.mods[index].enabled { return Ok(()); }
        if let Some(dependent) = self.mods.iter().find(|item| item.enabled && item.manifest.dependencies.iter().any(|dependency| dependency.id == id)) {
            return Err(format!("cannot disable `{id}` while dependent mod `{}` is enabled", dependent.manifest.id));
        }
        let api = self.api_for(index);
        let result = call_lifecycle(self.mods[index].loaded.as_ref(), Lifecycle::Disable, api);
        self.mods[index].enabled = false;
        self.saved_enabled.insert(id.to_string(), false);
        self.save_enabled_state();
        if let Err(err) = result {
            self.mods[index].last_error = Some(err.clone());
            return Err(err);
        }
        self.mods[index].last_error = None;
        Ok(())
    }

    pub fn toggle(&mut self, id: &str) -> Result<bool, String> {
        let enabled = self.mods.get(self.index_of(id)?).map(|item| item.enabled).unwrap_or(false);
        if enabled { self.disable(id)?; } else { self.enable(id)?; }
        Ok(!enabled)
    }

    pub fn reload(&mut self, id: &str) -> Result<(), String> {
        let index = self.index_of(id)?;
        let was_enabled = self.mods[index].enabled;
        if was_enabled {
            let api = self.api_for(index);
            let _ = call_lifecycle(self.mods[index].loaded.as_ref(), Lifecycle::Disable, api);
        }
        self.mods[index].enabled = false;
        self.mods[index].loaded = None;
        let entry = safe_entry_path(&self.mods[index].directory, &self.mods[index].manifest.entry)?;
        self.mods[index].source_hash = hash_file(&entry)?;
        match load_mod(&self.mods[index].manifest, &self.mods[index].directory, self.limits) {
            Ok(loaded) => self.mods[index].loaded = Some(loaded),
            Err(err) => {
                self.mods[index].last_error = Some(err.clone());
                return Err(err);
            }
        }
        if was_enabled { self.enable(id)?; }
        self.mods[index].last_error = None;
        Ok(())
    }

    pub fn update(&mut self, dt: f32) {
        let mut state_changed = false;
        for index in 0..self.mods.len() {
            if !self.mods[index].enabled { continue; }
            let api = self.api_for(index);
            let result = call_update(self.mods[index].loaded.as_ref(), api.clone(), dt);
            if let Err(err) = result {
                // Give the mod a chance to release persistent game-side effects
                // before isolating it after a callback failure.
                let _ = call_lifecycle(self.mods[index].loaded.as_ref(), Lifecycle::Disable, api);
                let id = self.mods[index].manifest.id.clone();
                self.mods[index].last_error = Some(err.clone());
                self.mods[index].enabled = false;
                self.saved_enabled.insert(id.clone(), false);
                state_changed = true;
                if let Ok(mut logs) = self.logs.lock() {
                    logs.push(format!("[{id}] disabled after update error: {err}"));
                }
            }
        }
        if state_changed { self.save_enabled_state(); }
    }

    pub fn set_context_number(&mut self, key: impl Into<String>, value: f64) {
        self.set_context(key.into(), LuaModValue::Number(value));
    }
    pub fn set_context_bool(&mut self, key: impl Into<String>, value: bool) {
        self.set_context(key.into(), LuaModValue::Boolean(value));
    }
    pub fn set_context_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.set_context(key.into(), LuaModValue::Text(value.into()));
    }
    pub fn take_commands(&mut self) -> Vec<LuaModCommand> {
        self.commands.lock().map(|mut values| std::mem::take(&mut *values)).unwrap_or_default()
    }
    pub fn take_logs(&mut self) -> Vec<String> {
        self.logs.lock().map(|mut values| std::mem::take(&mut *values)).unwrap_or_default()
    }
    pub fn reload_changed(&mut self) -> Vec<(String, Result<(), String>)> {
        let changed = self.mods.iter().filter_map(|item| {
            let entry = safe_entry_path(&item.directory, &item.manifest.entry).ok()?;
            let hash = hash_file(&entry).ok()?;
            (hash != item.source_hash).then_some(item.manifest.id.clone())
        }).collect::<Vec<_>>();
        changed.into_iter().map(|id| {
            let result = self.reload(&id);
            (id, result)
        }).collect()
    }
    pub fn active_fingerprint(&self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for item in self.mods.iter().filter(|item| item.enabled) {
            for byte in format!("{}@{}#{:016x};", item.manifest.id, item.manifest.version, item.source_hash).bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }

    fn set_context(&mut self, key: String, value: LuaModValue) {
        if let Ok(mut context) = self.context.lock() { context.insert(key, value); }
    }
    fn index_of(&self, id: &str) -> Result<usize, String> {
        self.mods.iter().position(|item| item.manifest.id == id).ok_or_else(|| format!("unknown mod `{id}`"))
    }
    fn api_for(&self, index: usize) -> LuaModApi {
        LuaModApi {
            mod_id: self.mods[index].manifest.id.clone(),
            context: self.context.clone(),
            commands: self.commands.clone(),
            logs: self.logs.clone(),
            state: self.mod_state.clone(),
        }
    }
    fn save_enabled_state(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.saved_enabled) {
            if std::fs::write(self.root.join("enabled_mods.json"), &json).is_err() {
                if let Some(parent) = self.fallback_state_path.parent() { let _ = std::fs::create_dir_all(parent); }
                let _ = std::fs::write(&self.fallback_state_path, json);
            }
        }
    }
}
