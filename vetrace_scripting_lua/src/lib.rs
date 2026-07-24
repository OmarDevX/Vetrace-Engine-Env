//! Optional Lua scripting plugin for Vetrace.
//!
//! `mlua` lives only in this crate. `vetrace_core` exposes the generic
//! `ScriptingBackend` trait and resource store, but it does not know about Lua.
//! Normal gameplay scripts and sandboxed third-party mods intentionally remain
//! separate security models.

pub mod backend;
pub mod bindings;
pub mod components;
mod context;
pub mod diagnostics;
pub mod debugger;
pub mod input;
pub mod modding;
pub mod plugin;
pub mod resources;
mod runtime_api;
pub mod state;

pub use backend::{
    LuaScriptingBackend, attach_autoload_script, attach_loaded_script, detach_script,
    dispatch_collision_enter, dispatch_collision_exit, dispatch_script_event,
    fixed_update_scripts, load_script_from_file, load_script_from_file_as,
    load_scripts_from_dir, reload_script_from_file_as, shutdown_autoload_scripts,
    shutdown_entity_scripts, start_pending_autoload_scripts,
    start_pending_scripts, update_autoload_scripts, update_scripts,
};
pub use bindings::{
    ComponentCollectionProxy, DynamicComponentProxy, EngineHandle, EntityProxy, InputProxy,
    TransformProxy,
};
pub use components::{ScriptComponent, ScriptValue};
pub use debugger::{
    LuaDebugValue, LuaDebugVariable, LuaDebuggerCommand, LuaDebuggerController, LuaDebuggerEvent,
    LuaDebuggerHandle, LuaPausedState, LuaStackFrame,
};
pub use diagnostics::{
    LuaDiagnosticTarget, LuaDiagnostics, LuaRuntimeConfig, LuaScriptError,
};
pub use input::{LuaInputAction, LuaInputMap};
pub use modding::{
    LuaModCommand, LuaModDependency, LuaModInfo, LuaModLimits, LuaModManager,
    LuaModManifest, LuaModValue,
};
pub use plugin::LuaScriptingPlugin;
pub use resources::LuaProjectContext;
pub use state::{
    LoadedLuaScript, LuaAutoloadInstance, LuaPropertyDefinition, LuaScriptInstance,
    LuaScriptInstanceStatus, LuaScriptMeta, LuaScriptStyle, LuaScriptingState,
};
