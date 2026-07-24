use super::*;

pub(super) fn install_window_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let window = lua.create_table()?;
    window.set("size", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
            Ok((settings.width, settings.height))
        })
    })?)?;
    window.set("width", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<RenderSettings>().map_or(1280, |value| value.width))
        })
    })?)?;
    window.set("height", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<RenderSettings>().map_or(720, |value| value.height))
        })
    })?)?;
    window.set("set_cursor", lua.create_function(|_, (visible, grabbed): (bool, bool)| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
                settings.cursor_visible = visible;
                settings.cursor_grab = grabbed;
            }
            Ok(())
        })
    })?)?;
    window.set("set_title", lua.create_function(|_, title: String| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
                settings.title = title;
            }
            Ok(())
        })
    })?)?;
    env.set("Window", window)
}
