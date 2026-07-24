use std::collections::{HashMap, HashSet, VecDeque};

use mlua::{Function, Table};
use vetrace_core::backends::ScriptingBackend;
use vetrace_audio::{AudioLoadMode, AudioPlayState, AudioSource};
use vetrace_core::{Engine, Entity, Transform};
use vetrace_physics::PhysicsActorExt;
use vetrace_scene::load_scene_file;

use crate::bindings::{EngineHandle, EntityProxy, InputProxy};
use crate::components::ScriptComponent;
use crate::context::{
    clear_entity_handles, forget_entity_handles, remember_entity_handle, resolve_entity_target,
    scope_context, LuaCommand,
    LuaEntityTarget,
};
use crate::debugger::LuaDebuggerController;
use crate::diagnostics::{
    LuaDiagnosticTarget, LuaDiagnostics, LuaRuntimeConfig, LuaScriptError,
};
use crate::state::{
    script_value_to_lua, LuaScriptInstanceStatus, LuaScriptStyle, LuaScriptingState,
};
use crate::components::ScriptValue;

mod api;
mod callbacks;
mod command_executor;
mod errors;
mod lifecycle;
mod shutdown;

pub use api::*;

use callbacks::*;
use command_executor::*;
use errors::*;
use lifecycle::*;
use shutdown::*;


pub struct LuaScriptingBackend;

#[derive(Clone, Copy, Debug, Default)]
struct LuaFrameErrors(u32);

#[derive(Clone, Copy, Debug)]
enum EntityCallback {
    Ready,
    Update,
    FixedUpdate,
    Destroy,
}

impl EntityCallback {
    fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Update => "update",
            Self::FixedUpdate => "fixed_update",
            Self::Destroy => "destroy",
        }
    }
}

impl LuaScriptingBackend {
    pub fn new() -> Self { Self }
}

fn begin_debug_callback(
    state: &LuaScriptingState,
    path: &str,
    callback: &str,
    entity: Option<Entity>,
    table: &Table,
) -> Option<LuaDebuggerController> {
    let debugger = state.debugger.clone()?;
    debugger.enter_callback(path, callback, entity.map(|entity| entity.0));
    if let Err(error) = debugger.set_instance_table(&state.lua, table) {
        eprintln!("Lua debugger could not expose callback instance: {error}");
    }
    Some(debugger)
}

fn finish_debug_callback(
    state: &LuaScriptingState,
    debugger: Option<LuaDebuggerController>,
    result: &mlua::Result<()>,
) {
    let Some(debugger) = debugger else { return; };
    if let Err(error) = result {
        debugger.report_error(&state.lua, &error.to_string());
    }
    debugger.clear_instance_table(&state.lua);
    debugger.leave_callback();
}

impl ScriptingBackend for LuaScriptingBackend {
    fn attach_script(&mut self, engine: &mut Engine, entity: Entity, source: &str) {
        ensure_resources(engine);
        let name = format!("entity_{}_script", entity.0);
        with_state(engine, |engine, state| {
            match state.load_script(name.clone(), source.to_owned(), None) {
                Ok(()) => {
                    state.attach_loaded_script(entity, name.clone());
                    engine.raw_world_mut().insert(
                        entity,
                        ScriptComponent { script: name, ..ScriptComponent::default() },
                    );
                }
                Err(error) => record_error(
                    engine,
                    LuaDiagnosticTarget::Entity { entity, script: name },
                    "load",
                    error.to_string(),
                ),
            }
        });
    }

    fn on_update(&mut self, engine: &mut Engine, dt: f32) {
        ensure_resources(engine);
        if let Some(errors) = engine.get_resource_mut::<LuaFrameErrors>() {
            errors.0 = 0;
        }
        with_state(engine, |engine, state| {
            cleanup_dead_script_instances(engine, state);
            synchronize_script_components_inner(engine, state);
            start_pending_autoload_scripts_inner(engine, state);
            start_pending_scripts_inner(engine, state);
            update_autoload_scripts_inner(engine, state, dt);
            update_scripts_inner(engine, state, dt, false);
        });
    }
}
