use super::*;

mod audio;
mod components;
mod events;
mod pending;
mod physics;
mod scene;

use audio::*;
use components::*;
use events::*;
use pending::*;
use physics::*;
use scene::*;

pub(super) fn flush_lua_commands(
    engine: &mut Engine,
    state: &mut LuaScriptingState,
    commands: Vec<LuaCommand>,
) {
    let mut commands = VecDeque::from(commands);
    let mut spawned = HashMap::<u64, Entity>::new();
    while let Some(command) = commands.pop_front() {
        match command {
            LuaCommand::Spawn { request, name } => {
                spawn_entity(engine, request, name, &mut spawned);
            }
            LuaCommand::InstantiateScene { request, path } => {
                instantiate_scene(engine, request, path, &mut spawned);
            }
            LuaCommand::Destroy(entity) => {
                destroy_entity(engine, state, entity, &mut commands);
            }
            LuaCommand::ClearScene => clear_scene(engine, state),
            LuaCommand::Stop => engine.stop(),
            LuaCommand::SetPendingName { request, name } => {
                set_pending_name(engine, &spawned, request, name);
            }
            LuaCommand::AddPendingTag { request, tag } => {
                add_pending_tag(engine, &spawned, request, tag);
            }
            LuaCommand::SetPendingTranslation { request, value } => {
                if let Some(entity) = resolve_request(engine, &spawned, request) {
                    set_actor_translation(engine, entity, value, false);
                }
            }
            LuaCommand::SetPendingRotation { request, value } => {
                if let Some(entity) = resolve_request(engine, &spawned, request) {
                    set_actor_rotation(engine, entity, value);
                }
            }
            LuaCommand::TranslatePending { request, value } => {
                if let Some(entity) = resolve_request(engine, &spawned, request) {
                    set_actor_translation(engine, entity, value, true);
                }
            }
            LuaCommand::SetPendingScale { request, value } => {
                if let Some(entity) = resolve_request(engine, &spawned, request) {
                    set_actor_scale(engine, entity, value);
                }
            }
            LuaCommand::AddComponent {
                target,
                component,
                value,
            } => add_component(engine, &spawned, target, component, value),
            LuaCommand::RemoveComponent { target, component } => {
                remove_component(engine, &spawned, target, component);
            }
            LuaCommand::SetVelocity { target, value } => {
                set_velocity(engine, &spawned, target, value);
            }
            LuaCommand::ApplyImpulse { target, value } => {
                apply_impulse(engine, &spawned, target, value);
            }
            LuaCommand::PlayAudio {
                request,
                path,
                position,
                volume,
                looping,
            } => play_audio(
                engine,
                &mut spawned,
                request,
                path,
                position,
                volume,
                looping,
            ),
            LuaCommand::EmitEvent {
                target,
                name,
                payload,
            } => emit_event(engine, state, target, name, payload, &mut commands),
        }
    }
}

pub(super) fn resolve_request(
    engine: &Engine,
    spawned: &HashMap<u64, Entity>,
    request: u64,
) -> Option<Entity> {
    spawned
        .get(&request)
        .copied()
        .or_else(|| resolve_entity_target(engine, LuaEntityTarget::Pending(request)))
}

pub(super) fn resolve_command_target(
    engine: &Engine,
    target: LuaEntityTarget,
    spawned: &HashMap<u64, Entity>,
) -> Option<Entity> {
    match target {
        LuaEntityTarget::Live(entity) => Some(entity),
        LuaEntityTarget::Pending(request) => resolve_request(engine, spawned, request),
    }
}

pub(super) fn set_actor_translation(
    engine: &mut Engine,
    entity: Entity,
    value: glam::Vec3,
    relative: bool,
) {
    let Some(actor) = engine.actor(entity) else {
        return;
    };
    if !actor.has::<Transform>(engine) {
        let _ = actor.insert(engine, Transform::default());
    }
    if let Some(transform) = actor.transform_mut(engine) {
        if relative {
            transform.translation += value;
        } else {
            transform.translation = value;
        }
    }
}

pub(super) fn set_actor_rotation(engine: &mut Engine, entity: Entity, value: glam::Quat) {
    let Some(actor) = engine.actor(entity) else {
        return;
    };
    if !actor.has::<Transform>(engine) {
        let _ = actor.insert(engine, Transform::default());
    }
    if let Some(transform) = actor.transform_mut(engine) {
        transform.rotation = if value.is_finite() && value.length_squared() > f32::EPSILON {
            value.normalize()
        } else {
            glam::Quat::IDENTITY
        };
    }
}

pub(super) fn set_actor_scale(engine: &mut Engine, entity: Entity, value: glam::Vec3) {
    let Some(actor) = engine.actor(entity) else {
        return;
    };
    if !actor.has::<Transform>(engine) {
        let _ = actor.insert(engine, Transform::default());
    }
    if let Some(transform) = actor.transform_mut(engine) {
        transform.scale = value;
    }
}
