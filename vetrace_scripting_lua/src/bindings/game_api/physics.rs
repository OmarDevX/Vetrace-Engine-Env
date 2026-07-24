use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let physics = lua.create_table()?;
    physics.set("velocity", lua.create_function(|_, entity: AnyUserData| {
        let proxy = entity.borrow::<EntityProxy>()?;
        let Some(entity) = resolve_entity_now(proxy.target)? else {
            return Ok((0.0_f32, 0.0_f32, 0.0_f32));
        };
        with_context(|engine, _, _, _, _, _| {
            let velocity = engine.actor(entity).map(|actor| actor.velocity(engine)).unwrap_or(glam::Vec3::ZERO);
            Ok((velocity.x, velocity.y, velocity.z))
        })
    })?)?;
    physics.set("set_velocity", lua.create_function(
        |_, (entity, x_or_vector, y, z): (AnyUserData, Value, Option<f32>, Option<f32>)| {
            let proxy = entity.borrow::<EntityProxy>()?;
            let value = parse_vec3_argument(x_or_vector, y, z, "Physics.set_velocity")?;
            queue_command(LuaCommand::SetVelocity {
                target: proxy.target,
                value,
            })
        },
    )?)?;
    physics.set("apply_impulse", lua.create_function(
        |_, (entity, x_or_vector, y, z): (AnyUserData, Value, Option<f32>, Option<f32>)| {
            let proxy = entity.borrow::<EntityProxy>()?;
            let value = parse_vec3_argument(x_or_vector, y, z, "Physics.apply_impulse")?;
            queue_command(LuaCommand::ApplyImpulse {
                target: proxy.target,
                value,
            })
        },
    )?)?;
    physics.set("set_enabled", lua.create_function(|_, (entity, enabled): (AnyUserData, bool)| {
        let proxy = entity.borrow::<EntityProxy>()?;
        let Some(entity) = resolve_entity_now(proxy.target)? else { return Ok(false); };
        with_context(|engine, _, _, _, _, _| {
            let Some(state) = engine.get_resource_mut::<vetrace_physics::PhysicsState>() else {
                return Ok(false);
            };
            let mut found = false;
            if let Some(handle) = state.entity_bodies.get(&entity).copied() {
                if let Some(body) = state.bodies.get_mut(handle) {
                    body.set_enabled(enabled);
                    if enabled { body.wake_up(true); }
                    found = true;
                }
            }
            if let Some(handle) = state.entity_colliders.get(&entity).copied() {
                if let Some(collider) = state.colliders.get_mut(handle) {
                    collider.set_enabled(enabled);
                    found = true;
                }
            }
            Ok(found)
        })
    })?)?;
    physics.set("is_enabled", lua.create_function(|_, entity: AnyUserData| {
        let proxy = entity.borrow::<EntityProxy>()?;
        let Some(entity) = resolve_entity_now(proxy.target)? else { return Ok(false); };
        with_context(|engine, _, _, _, _, _| {
            let Some(state) = engine.get_resource::<vetrace_physics::PhysicsState>() else {
                return Ok(false);
            };
            let mut found = false;
            let mut enabled = true;
            if let Some(handle) = state.entity_bodies.get(&entity).copied() {
                if let Some(body) = state.bodies.get(handle) {
                    enabled &= body.is_enabled();
                    found = true;
                }
            }
            if let Some(handle) = state.entity_colliders.get(&entity).copied() {
                if let Some(collider) = state.colliders.get(handle) {
                    enabled &= collider.is_enabled();
                    found = true;
                }
            }
            Ok(found && enabled)
        })
    })?)?;
    physics.set("raycast", lua.create_function(
        |lua,
         (a, b, c, d, e, f, g): (
            Value,
            Value,
            Option<Value>,
            Option<Value>,
            Option<Value>,
            Option<Value>,
            Option<Value>,
        )| {
            let (origin, direction, max_distance) =
                parse_raycast_arguments(a, b, c, d, e, f, g)?;
            with_context(|engine, _, _, _, _, _| {
                let Some(hit) = raycast_colliders(
                    engine,
                    origin,
                    direction,
                    max_distance.unwrap_or(1000.0),
                    |_| true,
                ) else {
                    return Ok(Value::Nil);
                };
                let result = lua.create_table()?;
                result.set("distance", hit.distance)?;
                result.set("position", vec3_to_lua_table(lua, hit.position)?)?;
                if let Some(entity) = hit.entity {
                    result.set("entity", lua.create_userdata(EntityProxy::live(entity))?)?;
                }
                Ok(Value::Table(result))
            })
        },
    )?)?;
    physics.set("gravity", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<vetrace_physics::PhysicsState>()
                .map(|state| (state.gravity.x, state.gravity.y, state.gravity.z))
                .unwrap_or((0.0, -9.81, 0.0)))
        })
    })?)?;
    physics.set("set_gravity", lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
        with_context(|engine, _, _, _, _, _| {
            let state = engine.get_resource_mut::<vetrace_physics::PhysicsState>()
                .ok_or_else(|| mlua::Error::external("physics state is unavailable"))?;
            state.gravity.x = x;
            state.gravity.y = y;
            state.gravity.z = z;
            Ok(())
        })
    })?)?;
    env.set("Physics", physics)?;
    Ok(())
}
