use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let input = lua.create_table()?;
    input.set("key_down", lua.create_function(|_, key: String| input_key_down(&key))?)?;
    input.set("key_pressed", lua.create_function(|_, key: String| input_key_pressed(&key))?)?;
    input.set("key_released", lua.create_function(|_, key: String| input_key_released(&key))?)?;
    input.set("action_down", lua.create_function(|_, action: String| input_action_down(&action))?)?;
    input.set("action_pressed", lua.create_function(|_, action: String| input_action_pressed(&action))?)?;
    input.set("action_released", lua.create_function(|_, action: String| input_action_released(&action))?)?;
    input.set("mouse_button_down", lua.create_function(|_, button: String| {
        with_input(|input| input.is_mouse_button_down(&button))
    })?)?;
    input.set("mouse_button_pressed", lua.create_function(|_, button: String| {
        with_input(|input| input.was_mouse_button_pressed(&button))
    })?)?;
    input.set("mouse_button_released", lua.create_function(|_, button: String| {
        with_input(|input| input.was_mouse_button_released(&button))
    })?)?;
    input.set("mouse_position", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<InputState>().map(InputState::mouse_position).unwrap_or_default())
        })
    })?)?;
    input.set("mouse_delta", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<InputState>().map(InputState::mouse_delta).unwrap_or_default())
        })
    })?)?;
    input.set("mouse_wheel_delta", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<InputState>().map(InputState::mouse_wheel_delta).unwrap_or_default())
        })
    })?)?;
    input.set("text_input", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<InputState>().map(|input| input.text_input().to_owned()).unwrap_or_default())
        })
    })?)?;
    env.set("Input", input)?;
    Ok(())
}
