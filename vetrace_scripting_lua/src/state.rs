use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use mlua::{Function, Lua, RegistryKey, Table, Value};
use vetrace_core::Entity;

use crate::bindings::{install_entity_component_api, install_game_api, EntityProxy, TransformProxy};
use crate::components::ScriptValue;
use crate::debugger::{LuaDebuggerController, LuaDebuggerHandle};

#[derive(Clone, Debug)]
pub struct LuaScriptMeta {
    pub name: String,
    pub path: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LuaScriptStyle {
    /// New Vetrace gameplay API: `ready(self)`, `update(self, dt)`, ...
    Gameplay,
    /// Compatibility API used by earlier projects: `start(engine, entity)`, ...
    Legacy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaPropertyDefinition {
    pub value_type: String,
    pub default: ScriptValue,
}

#[derive(Default)]
pub struct LuaCallbacks {
    pub ready: Option<RegistryKey>,
    pub update: Option<RegistryKey>,
    pub fixed_update: Option<RegistryKey>,
    pub destroy: Option<RegistryKey>,
    pub event: Option<RegistryKey>,
    pub collision_enter: Option<RegistryKey>,
    pub collision_exit: Option<RegistryKey>,
    pub legacy_collision: Option<RegistryKey>,
}

pub struct LoadedLuaScript {
    pub meta: LuaScriptMeta,
    pub source: String,
    /// Returned script table used as the read-only prototype for every
    /// per-entity/autoload instance table.
    pub template: RegistryKey,
    pub style: LuaScriptStyle,
    pub callbacks: LuaCallbacks,
    pub properties: BTreeMap<String, LuaPropertyDefinition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LuaScriptInstanceStatus {
    Pending,
    Running,
    Failed,
    Disabled,
}

pub struct LuaScriptInstance {
    pub script: String,
    pub table: RegistryKey,
    pub status: LuaScriptInstanceStatus,
    pub last_error: Option<String>,
    pub error_count: u32,
}

pub struct LuaAutoloadInstance {
    pub table: RegistryKey,
    pub status: LuaScriptInstanceStatus,
    pub last_error: Option<String>,
    pub error_count: u32,
}

#[derive(Default)]
pub struct LuaScriptingState {
    pub lua: Lua,
    pub scripts: HashMap<String, LoadedLuaScript>,
    /// Compatibility/public lookup used by the runtime and diagnostics.
    pub entity_scripts: HashMap<Entity, String>,
    /// Compatibility/public lifecycle set retained for existing callers.
    pub started_scripts: HashSet<Entity>,
    pub autoload_scripts: Vec<String>,
    pub started_autoload_scripts: HashSet<String>,
    pub instances: HashMap<Entity, LuaScriptInstance>,
    pub autoload_instances: HashMap<String, LuaAutoloadInstance>,
    pub next_spawn_request: u64,
    pub debugger: Option<LuaDebuggerController>,
}

impl LuaScriptingState {
    pub fn new() -> Self {
        Self {
            lua: Lua::new(),
            scripts: HashMap::new(),
            entity_scripts: HashMap::new(),
            started_scripts: HashSet::new(),
            autoload_scripts: Vec::new(),
            started_autoload_scripts: HashSet::new(),
            instances: HashMap::new(),
            autoload_instances: HashMap::new(),
            next_spawn_request: 1,
            debugger: None,
        }
    }

    pub fn enable_debugger(&mut self) -> mlua::Result<LuaDebuggerHandle> {
        let handle = LuaDebuggerController::install(&self.lua)?;
        self.debugger = Some(handle.controller());
        Ok(handle)
    }

    pub fn disable_debugger(&mut self) {
        self.lua.remove_hook();
        self.debugger = None;
    }

    pub fn load_script_from_file(&mut self, path: impl AsRef<Path>) -> mlua::Result<String> {
        let path = path.as_ref();
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("script")
            .to_string();
        self.load_script_from_file_as(path, name)
    }

    pub fn load_script_from_file_as(
        &mut self,
        path: impl AsRef<Path>,
        name: impl Into<String>,
    ) -> mlua::Result<String> {
        let path = path.as_ref();
        let name = name.into();
        let source = std::fs::read_to_string(path).map_err(mlua::Error::external)?;
        self.load_script(name.clone(), source, Some(path.to_path_buf()))?;
        Ok(name)
    }

    pub fn load_scripts_from_dir(&mut self, dir: impl AsRef<Path>) -> mlua::Result<Vec<String>> {
        let mut loaded = Vec::new();
        let dir = dir.as_ref();
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(loaded),
            Err(err) => return Err(mlua::Error::external(err)),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("lua") {
                loaded.push(self.load_script_from_file(path)?);
            }
        }
        Ok(loaded)
    }

    pub fn load_script(
        &mut self,
        name: String,
        source: String,
        path: Option<PathBuf>,
    ) -> mlua::Result<()> {
        let env = self.lua.create_table()?;
        copy_globals(&self.lua, &env)?;
        install_game_api(&self.lua, &env)?;
        let value = self
            .lua
            .load(&source)
            .set_name(&name)
            .set_environment(env.clone())
            .eval::<Value>()?;

        let table = match value {
            Value::Table(table) => table,
            _ => env,
        };

        let properties = parse_property_definitions(&table)?;
        let has_start = has_function(&table, "start")?;
        let has_ready = has_function(&table, "ready")?;
        let has_gameplay_only_callback = has_function(&table, "fixed_update")?
            || has_function(&table, "destroy")?
            || has_function(&table, "on_event")?
            || has_function(&table, "on_collision_enter")?
            || has_function(&table, "on_collision_exit")?;
        // Legacy scripts are detected narrowly so a new script containing only
        // `update(self, dt)` still uses the gameplay API. Existing Vetrace Lua
        // scripts used `start(engine, ...)`, which remains an unambiguous
        // compatibility marker.
        let style = if has_start
            && !has_ready
            && properties.is_empty()
            && !has_gameplay_only_callback
        {
            LuaScriptStyle::Legacy
        } else {
            LuaScriptStyle::Gameplay
        };

        let mut callbacks = LuaCallbacks::default();
        callbacks.ready = registry_fn(&self.lua, &table, if style == LuaScriptStyle::Gameplay { "ready" } else { "start" })?;
        if callbacks.ready.is_none() && style == LuaScriptStyle::Gameplay {
            callbacks.ready = registry_fn(&self.lua, &table, "start")?;
        }
        callbacks.update = registry_fn(&self.lua, &table, "update")?;
        callbacks.fixed_update = registry_fn(&self.lua, &table, "fixed_update")?;
        callbacks.destroy = registry_fn(&self.lua, &table, "destroy")?;
        callbacks.event = registry_fn(&self.lua, &table, "on_event")?;
        callbacks.collision_enter = registry_fn(&self.lua, &table, "on_collision_enter")?;
        callbacks.collision_exit = registry_fn(&self.lua, &table, "on_collision_exit")?;
        callbacks.legacy_collision = registry_fn(&self.lua, &table, "on_collision")?;

        let template = self.lua.create_registry_value(table)?;
        self.scripts.insert(
            name.clone(),
            LoadedLuaScript {
                meta: LuaScriptMeta { name, path },
                source,
                template,
                style,
                callbacks,
                properties,
            },
        );
        Ok(())
    }

    pub fn attach_loaded_script(&mut self, entity: Entity, name: impl Into<String>) {
        self.entity_scripts.insert(entity, name.into());
        self.started_scripts.remove(&entity);
        self.instances.remove(&entity);
    }

    pub fn attach_autoload_script(&mut self, name: impl Into<String>) {
        let name = name.into();
        if !self.autoload_scripts.iter().any(|existing| existing == &name) {
            self.autoload_scripts.push(name.clone());
        }
        self.started_autoload_scripts.remove(&name);
        self.autoload_instances.remove(&name);
    }

    pub fn create_entity_instance(
        &mut self,
        entity: Entity,
        script_name: &str,
        overrides: &BTreeMap<String, ScriptValue>,
    ) -> mlua::Result<()> {
        let script = self.scripts.get(script_name).ok_or_else(|| {
            mlua::Error::external(format!("Lua script '{script_name}' is not loaded"))
        })?;
        let template = self.lua.registry_value::<Table>(&script.template)?;
        let table = self.lua.create_table()?;
        install_instance_prototype(&self.lua, &table, template)?;
        table.set("entity", self.lua.create_userdata(EntityProxy::live(entity))?)?;
        table.set("transform", self.lua.create_userdata(TransformProxy::live(entity))?)?;
        install_entity_component_api(&self.lua, &table, entity)?;
        table.set("script", script_name)?;

        for (name, definition) in &script.properties {
            table.set(name.as_str(), script_value_to_lua(&self.lua, &definition.default)?)?;
        }
        for (name, value) in overrides {
            table.set(name.as_str(), script_value_to_lua(&self.lua, value)?)?;
        }

        let key = self.lua.create_registry_value(table)?;
        self.instances.insert(
            entity,
            LuaScriptInstance {
                script: script_name.to_owned(),
                table: key,
                status: LuaScriptInstanceStatus::Pending,
                last_error: None,
                error_count: 0,
            },
        );
        Ok(())
    }

    pub fn create_autoload_instance(&mut self, script_name: &str) -> mlua::Result<()> {
        let script = self.scripts.get(script_name).ok_or_else(|| {
            mlua::Error::external(format!("Lua script '{script_name}' is not loaded"))
        })?;
        let template = self.lua.registry_value::<Table>(&script.template)?;
        let table = self.lua.create_table()?;
        install_instance_prototype(&self.lua, &table, template)?;
        table.set("script", script_name)?;
        for (name, definition) in &script.properties {
            table.set(name.as_str(), script_value_to_lua(&self.lua, &definition.default)?)?;
        }
        let key = self.lua.create_registry_value(table)?;
        self.autoload_instances.insert(
            script_name.to_owned(),
            LuaAutoloadInstance {
                table: key,
                status: LuaScriptInstanceStatus::Pending,
                last_error: None,
                error_count: 0,
            },
        );
        Ok(())
    }

    pub fn detach_entity(&mut self, entity: Entity) {
        self.entity_scripts.remove(&entity);
        self.started_scripts.remove(&entity);
        self.instances.remove(&entity);
    }

    pub fn remove_dead_entities(&mut self, alive: impl Fn(Entity) -> bool) {
        self.entity_scripts.retain(|entity, _| alive(*entity));
        self.started_scripts.retain(|entity| alive(*entity));
        self.instances.retain(|entity, _| alive(*entity));
    }
}

fn install_instance_prototype(lua: &Lua, instance: &Table, template: Table) -> mlua::Result<()> {
    let metatable = lua.create_table()?;
    metatable.set("__index", template)?;
    instance.set_metatable(Some(metatable))?;
    Ok(())
}

fn copy_globals(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let globals = lua.globals();
    env.set("_G", env.clone())?;
    env.set("print", globals.get::<Function>("print")?)?;
    env.set("pairs", globals.get::<Function>("pairs")?)?;
    env.set("ipairs", globals.get::<Function>("ipairs")?)?;
    env.set("type", globals.get::<Function>("type")?)?;
    env.set("tostring", globals.get::<Function>("tostring")?)?;
    env.set("tonumber", globals.get::<Function>("tonumber")?)?;
    env.set("error", globals.get::<Function>("error")?)?;
    env.set("assert", globals.get::<Function>("assert")?)?;
    env.set("pcall", globals.get::<Function>("pcall")?)?;
    env.set("xpcall", globals.get::<Function>("xpcall")?)?;
    env.set("next", globals.get::<Function>("next")?)?;
    env.set("select", globals.get::<Function>("select")?)?;
    env.set("rawequal", globals.get::<Function>("rawequal")?)?;
    env.set("rawget", globals.get::<Function>("rawget")?)?;
    env.set("rawset", globals.get::<Function>("rawset")?)?;
    env.set("rawlen", globals.get::<Function>("rawlen")?)?;
    env.set("math", globals.get::<Table>("math")?)?;
    env.set("string", globals.get::<Table>("string")?)?;
    env.set("table", globals.get::<Table>("table")?)?;
    if let Ok(value) = globals.get::<Table>("utf8") { env.set("utf8", value)?; }
    if let Ok(value) = globals.get::<Table>("coroutine") { env.set("coroutine", value)?; }
    Ok(())
}

fn has_function(table: &Table, name: &str) -> mlua::Result<bool> {
    Ok(table.get::<Option<Function>>(name)?.is_some())
}

fn registry_fn(lua: &Lua, table: &Table, name: &str) -> mlua::Result<Option<RegistryKey>> {
    match table.get::<Option<Function>>(name)? {
        Some(function) => Ok(Some(lua.create_registry_value(function)?)),
        None => Ok(None),
    }
}

fn parse_property_definitions(table: &Table) -> mlua::Result<BTreeMap<String, LuaPropertyDefinition>> {
    let Some(properties) = table.get::<Option<Table>>("properties")? else {
        return Ok(BTreeMap::new());
    };
    let mut definitions = BTreeMap::new();
    for pair in properties.pairs::<String, Value>() {
        let (name, value) = pair?;
        let (value_type, default) = match value {
            Value::Table(descriptor) => {
                let explicit_type = descriptor.get::<Option<String>>("type")?;
                let default_value = descriptor.get::<Option<Value>>("default")?.unwrap_or(Value::Nil);
                let default = script_value_from_lua(default_value)?;
                (explicit_type.unwrap_or_else(|| default.type_name().to_owned()), default)
            }
            other => {
                let default = script_value_from_lua(other)?;
                (default.type_name().to_owned(), default)
            }
        };
        definitions.insert(name, LuaPropertyDefinition { value_type, default });
    }
    Ok(definitions)
}

pub(crate) fn script_value_to_lua<'lua>(lua: &'lua Lua, value: &ScriptValue) -> mlua::Result<Value> {
    match value {
        ScriptValue::Bool(value) => Ok(Value::Boolean(*value)),
        ScriptValue::Integer(value) => Ok(Value::Integer(*value)),
        ScriptValue::Number(value) => Ok(Value::Number(*value)),
        ScriptValue::String(value) => Ok(Value::String(lua.create_string(value.as_str())?)),
        ScriptValue::Vec2(value) => vector_table(lua, value),
        ScriptValue::Vec3(value) => vector_table(lua, value),
        ScriptValue::Vec4(value) => vector_table(lua, value),
        ScriptValue::Null => Ok(Value::Nil),
    }
}

fn vector_table<const N: usize>(lua: &Lua, values: &[f32; N]) -> mlua::Result<Value> {
    let table = lua.create_table()?;
    for (index, value) in values.iter().enumerate() {
        table.set(index + 1, *value)?;
    }
    Ok(Value::Table(table))
}

fn script_value_from_lua(value: Value) -> mlua::Result<ScriptValue> {
    match value {
        Value::Nil => Ok(ScriptValue::Null),
        Value::Boolean(value) => Ok(ScriptValue::Bool(value)),
        Value::Integer(value) => Ok(ScriptValue::Integer(value)),
        Value::Number(value) => Ok(ScriptValue::Number(value)),
        Value::String(value) => Ok(ScriptValue::String(value.to_string_lossy().to_string())),
        Value::Table(table) => {
            let length = table.raw_len();
            let mut values = Vec::with_capacity(length);
            for index in 1..=length {
                let value = table.get::<f32>(index)?;
                values.push(value);
            }
            match values.as_slice() {
                [x, y] => Ok(ScriptValue::Vec2([*x, *y])),
                [x, y, z] => Ok(ScriptValue::Vec3([*x, *y, *z])),
                [x, y, z, w] => Ok(ScriptValue::Vec4([*x, *y, *z, *w])),
                _ => Err(mlua::Error::external(
                    "script property tables must be vectors with 2, 3, or 4 numeric values",
                )),
            }
        }
        other => Err(mlua::Error::external(format!(
            "unsupported script property value: {other:?}"
        ))),
    }
}
