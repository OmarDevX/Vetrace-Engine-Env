use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LuaModManifest {
    pub id: String,
    pub name: String,
    #[serde(default = "default_mod_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default = "default_mod_entry")]
    pub entry: String,
    #[serde(default)]
    pub enabled_by_default: bool,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<LuaModDependency>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LuaModDependency {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct LuaModLimits {
    pub memory_bytes: usize,
    pub instructions_per_callback: u64,
    pub hook_interval: u32,
    pub max_source_bytes: u64,
}

impl Default for LuaModLimits {
    fn default() -> Self {
        Self { memory_bytes: 16 * 1024 * 1024, instructions_per_callback: 500_000, hook_interval: 1_000, max_source_bytes: 1024 * 1024 }
    }
}

fn default_mod_version() -> String { "0.1.0".to_string() }
fn default_mod_entry() -> String { "main.lua".to_string() }

#[derive(Clone, Debug, PartialEq)]
pub enum LuaModValue {
    Number(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LuaModCommand {
    pub mod_id: String,
    pub name: String,
    pub value: LuaModValue,
}

#[derive(Clone, Debug)]
pub struct LuaModInfo {
    pub manifest: LuaModManifest,
    pub enabled: bool,
    pub loaded: bool,
    pub last_error: Option<String>,
}

