//! Small, manifest-driven Lua mod host.
//!
//! Mods run in isolated Lua states and communicate through a deliberately
//! narrow value/command API. Games decide which context keys and commands are
//! meaningful, keeping game policy out of the generic scripting crate.

use std::collections::{BTreeMap, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mlua::{Function, HookTriggers, Lua, RegistryKey, Table, UserData, UserDataMethods, Value, VmState};

mod api;
mod manager;
mod manifest;
mod runtime;

use api::*;
pub use manager::LuaModManager;
pub use manifest::{LuaModCommand, LuaModDependency, LuaModInfo, LuaModLimits, LuaModManifest, LuaModValue};
use runtime::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_manifest_entry_traversal() {
        let manifest = LuaModManifest {
            id: "bad".to_string(), name: "Bad".to_string(), version: "1".to_string(),
            description: String::new(), author: String::new(), entry: "../bad.lua".to_string(), enabled_by_default: false,
            capabilities: Vec::new(), dependencies: Vec::new(), conflicts: Vec::new(), priority: 0,
        };
        assert!(validate_manifest(&manifest).is_err());
    }

    #[test]
    fn isolated_mod_emits_commands() {
        let manifest = LuaModManifest {
            id: "test".to_string(), name: "Test".to_string(), version: "1".to_string(),
            description: String::new(), author: String::new(), entry: "main.lua".to_string(), enabled_by_default: false,
            capabilities: Vec::new(), dependencies: Vec::new(), conflicts: Vec::new(), priority: 0,
        };
        let dir = std::env::temp_dir().join(format!("vetrace_lua_mod_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("main.lua"), "return { on_enable = function(api) api:emit_number('speed', 1.5) end }").unwrap();
        let loaded = load_mod(&manifest, &dir, LuaModLimits::default()).unwrap();
        let commands = Arc::new(Mutex::new(Vec::new()));
        call_lifecycle(Some(&loaded), Lifecycle::Enable, LuaModApi {
            mod_id: "test".to_string(), context: Arc::new(Mutex::new(HashMap::new())), commands: commands.clone(), logs: Arc::new(Mutex::new(Vec::new())), state: Arc::new(Mutex::new(HashMap::new())),
        }).unwrap();
        assert_eq!(commands.lock().unwrap()[0].value, LuaModValue::Number(1.5));
        let _ = std::fs::remove_dir_all(dir);
    }
}
