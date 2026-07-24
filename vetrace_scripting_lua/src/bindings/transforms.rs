use super::*;

pub(super) fn rotation(target: LuaEntityTarget) -> mlua::Result<(f32, f32, f32, f32)> {
    let transform = read_transform(target)?;
    Ok((transform.rotation.x, transform.rotation.y, transform.rotation.z, transform.rotation.w))
}

pub(super) fn set_rotation(target: LuaEntityTarget, value: glam::Quat) -> mlua::Result<()> {
    match target {
        LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
            ensure_transform(engine, entity)?.rotation = value;
            Ok(())
        }),
        LuaEntityTarget::Pending(request) => queue_command(LuaCommand::SetPendingRotation { request, value }),
    }
}

pub(super) fn translation(target: LuaEntityTarget) -> mlua::Result<(f32, f32, f32)> {
    let translation = read_transform(target)?.translation;
    Ok((translation.x, translation.y, translation.z))
}

pub(super) fn scale(target: LuaEntityTarget) -> mlua::Result<(f32, f32, f32)> {
    let scale = read_transform(target)?.scale;
    Ok((scale.x, scale.y, scale.z))
}

pub(super) fn set_translation(target: LuaEntityTarget, value: glam::Vec3) -> mlua::Result<()> {
    match target {
        LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
            ensure_transform(engine, entity)?.translation = value;
            Ok(())
        }),
        LuaEntityTarget::Pending(request) => queue_command(LuaCommand::SetPendingTranslation { request, value }),
    }
}

pub(super) fn translate(target: LuaEntityTarget, value: glam::Vec3) -> mlua::Result<()> {
    match target {
        LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
            ensure_transform(engine, entity)?.translation += value;
            Ok(())
        }),
        LuaEntityTarget::Pending(request) => queue_command(LuaCommand::TranslatePending { request, value }),
    }
}

pub(super) fn set_scale(target: LuaEntityTarget, value: glam::Vec3) -> mlua::Result<()> {
    match target {
        LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
            ensure_transform(engine, entity)?.scale = value;
            Ok(())
        }),
        LuaEntityTarget::Pending(request) => queue_command(LuaCommand::SetPendingScale { request, value }),
    }
}

pub(super) fn read_transform(target: LuaEntityTarget) -> mlua::Result<Transform> {
    with_context(|engine, _, _, _, _, _| {
        let Some(entity) = resolve_entity_target(engine, target) else {
            return Ok(Transform::default());
        };
        Ok(engine
            .actor(entity)
            .and_then(|actor| actor.transform(engine))
            .cloned()
            .unwrap_or_default())
    })
}

pub(super) fn ensure_transform<'a>(engine: &'a mut vetrace_core::Engine, entity: Entity) -> mlua::Result<&'a mut Transform> {
    let Some(actor) = engine.actor(entity) else {
        return Err(mlua::Error::external(format!("entity {} is no longer alive", entity.0)));
    };
    if !actor.has::<Transform>(engine) {
        actor.insert(engine, Transform::default()).map_err(mlua::Error::external)?;
    }
    actor
        .transform_mut(engine)
        .ok_or_else(|| mlua::Error::external("failed to access entity transform"))
}
