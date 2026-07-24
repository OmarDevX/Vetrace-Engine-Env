use super::*;

pub(super) fn with_input(operation: impl FnOnce(&InputState) -> bool) -> mlua::Result<bool> {
    with_context(|engine, _, _, _, _, _| {
        Ok(engine.get_resource::<InputState>().is_some_and(operation))
    })
}

pub(super) fn input_key_down(key: &str) -> mlua::Result<bool> { with_input(|input| input.is_key_down(key)) }
pub(super) fn input_key_pressed(key: &str) -> mlua::Result<bool> { with_input(|input| input.was_key_pressed(key)) }
pub(super) fn input_key_released(key: &str) -> mlua::Result<bool> { with_input(|input| input.was_key_released(key)) }

pub(super) fn input_action_down(action: &str) -> mlua::Result<bool> {
    with_context(|engine, _, _, _, _, _| {
        let Some(input) = engine.get_resource::<InputState>() else { return Ok(false); };
        Ok(engine
            .get_resource::<LuaInputMap>()
            .is_some_and(|map| map.action_down(input, action)))
    })
}

pub(super) fn input_action_pressed(action: &str) -> mlua::Result<bool> {
    with_context(|engine, _, _, _, _, _| {
        let Some(input) = engine.get_resource::<InputState>() else { return Ok(false); };
        Ok(engine
            .get_resource::<LuaInputMap>()
            .is_some_and(|map| map.action_pressed(input, action)))
    })
}

pub(super) fn input_action_released(action: &str) -> mlua::Result<bool> {
    with_context(|engine, _, _, _, _, _| {
        let Some(input) = engine.get_resource::<InputState>() else { return Ok(false); };
        Ok(engine
            .get_resource::<LuaInputMap>()
            .is_some_and(|map| map.action_released(input, action)))
    })
}
