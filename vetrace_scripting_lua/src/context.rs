use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::NonNull;

use vetrace_core::{DynamicValue, Engine, Entity};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LuaEntityTarget {
    Live(Entity),
    Pending(u64),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LuaEntityHandles {
    entities: HashMap<u64, Entity>,
}

pub(crate) fn remember_entity_handle(engine: &mut Engine, request: u64, entity: Entity) {
    if let Some(handles) = engine.get_resource_mut::<LuaEntityHandles>() {
        handles.entities.insert(request, entity);
    } else {
        let mut handles = LuaEntityHandles::default();
        handles.entities.insert(request, entity);
        engine.insert_resource(handles);
    }
}

pub(crate) fn resolve_entity_target(engine: &Engine, target: LuaEntityTarget) -> Option<Entity> {
    match target {
        LuaEntityTarget::Live(entity) => Some(entity),
        LuaEntityTarget::Pending(request) => engine
            .get_resource::<LuaEntityHandles>()
            .and_then(|handles| handles.entities.get(&request).copied()),
    }
}

pub(crate) fn forget_entity_handles(engine: &mut Engine, entity: Entity) {
    if let Some(handles) = engine.get_resource_mut::<LuaEntityHandles>() {
        handles.entities.retain(|_, mapped| *mapped != entity);
    }
}

pub(crate) fn clear_entity_handles(engine: &mut Engine) {
    if let Some(handles) = engine.get_resource_mut::<LuaEntityHandles>() {
        handles.entities.clear();
    }
}

#[derive(Clone, Debug)]
pub(crate) enum LuaCommand {
    Spawn { request: u64, name: String },
    InstantiateScene { request: u64, path: String },
    Destroy(Entity),
    ClearScene,
    Stop,
    SetPendingName { request: u64, name: String },
    AddPendingTag { request: u64, tag: String },
    SetPendingTranslation { request: u64, value: glam::Vec3 },
    SetPendingRotation { request: u64, value: glam::Quat },
    TranslatePending { request: u64, value: glam::Vec3 },
    SetPendingScale { request: u64, value: glam::Vec3 },
    AddComponent { target: LuaEntityTarget, component: String, value: Option<DynamicValue> },
    RemoveComponent { target: LuaEntityTarget, component: String },
    SetVelocity { target: LuaEntityTarget, value: glam::Vec3 },
    ApplyImpulse { target: LuaEntityTarget, value: glam::Vec3 },
    PlayAudio {
        request: u64,
        path: String,
        position: Option<glam::Vec3>,
        volume: f32,
        looping: bool,
    },
    EmitEvent {
        target: Option<Entity>,
        name: String,
        payload: crate::ScriptValue,
    },
}

/// Callback-only access to engine state.
///
/// The pointers are private, non-null, installed only for the duration of
/// `scope_context`, and restored even if Lua unwinds. Lua userdata never stores
/// either pointer. This keeps the unavoidable thread-local bridge contained in
/// one module and prevents callers from constructing or retaining contexts.
struct LuaCallbackContext {
    engine: NonNull<Engine>,
    commands: NonNull<Vec<LuaCommand>>,
    current_entity: Option<Entity>,
    delta_seconds: f32,
    fixed_update: bool,
    next_spawn_request: NonNull<u64>,
}

std::thread_local! {
    static ACTIVE_CONTEXT: RefCell<Option<LuaCallbackContext>> = const { RefCell::new(None) };
}

struct LuaContextGuard {
    previous: Option<LuaCallbackContext>,
}

impl Drop for LuaContextGuard {
    fn drop(&mut self) {
        ACTIVE_CONTEXT.with(|slot| {
            *slot.borrow_mut() = self.previous.take();
        });
    }
}

/// Runs `operation` with a callback-scoped engine bridge installed.
///
/// No context value is returned, so the borrowed engine/queues cannot escape
/// through the safe API. Nested callbacks restore the previous context.
pub(crate) fn scope_context<R>(
    engine: &mut Engine,
    commands: &mut Vec<LuaCommand>,
    current_entity: Option<Entity>,
    delta_seconds: f32,
    fixed_update: bool,
    next_spawn_request: &mut u64,
    operation: impl FnOnce() -> R,
) -> R {
    let context = LuaCallbackContext {
        engine: NonNull::from(engine),
        commands: NonNull::from(commands),
        current_entity,
        delta_seconds,
        fixed_update,
        next_spawn_request: NonNull::from(next_spawn_request),
    };
    let previous = ACTIVE_CONTEXT.with(|slot| slot.borrow_mut().replace(context));
    let _guard = LuaContextGuard { previous };
    operation()
}

pub(crate) fn with_context<R>(
    operation: impl FnOnce(&mut Engine, &mut Vec<LuaCommand>, &mut u64, Option<Entity>, f32, bool) -> mlua::Result<R>,
) -> mlua::Result<R> {
    ACTIVE_CONTEXT.with(|slot| {
        let mut borrowed = slot.borrow_mut();
        let Some(context) = borrowed.as_mut() else {
            return Err(mlua::Error::external(
                "Vetrace Lua API can only be used while a script callback is running",
            ));
        };

        // SAFETY: `scope_context` creates all pointers from live exclusive
        // references, keeps them private, and removes/restores the context
        // before those references can expire. The RefCell also rejects
        // re-entrant mutable access to this exact active context.
        unsafe {
            operation(
                context.engine.as_mut(),
                context.commands.as_mut(),
                context.next_spawn_request.as_mut(),
                context.current_entity,
                context.delta_seconds,
                context.fixed_update,
            )
        }
    })
}

pub(crate) fn queue_command(command: LuaCommand) -> mlua::Result<()> {
    with_context(|_, commands, _, _, _, _| {
        commands.push(command);
        Ok(())
    })
}

pub(crate) fn allocate_spawn_request() -> mlua::Result<u64> {
    with_context(|_, _, next_request, _, _, _| {
        let request = *next_request;
        *next_request = (*next_request).saturating_add(1);
        Ok(request)
    })
}
