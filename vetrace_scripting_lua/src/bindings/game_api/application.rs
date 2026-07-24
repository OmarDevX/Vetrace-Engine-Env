use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let application = lua.create_table()?;
    application.set("quit", lua.create_function(|_, ()| queue_command(LuaCommand::Stop))?)?;
    application.set("viewport_size", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            let settings = engine.get_resource::<RenderSettings>();
            Ok(settings.map(|settings| (settings.width as f32, settings.height as f32)).unwrap_or((1280.0, 720.0)))
        })
    })?)?;
    application.set("set_cursor_mode", lua.create_function(|_, (grab, visible): (bool, bool)| {
        with_context(|engine, _, _, _, _, _| {
            let settings = engine.get_resource_mut::<RenderSettings>()
                .ok_or_else(|| mlua::Error::external("render settings are unavailable"))?;
            settings.cursor_grab = grab;
            settings.cursor_visible = visible;
            Ok(())
        })
    })?)?;
    application.set("set_title", lua.create_function(|_, title: String| {
        with_context(|engine, _, _, _, _, _| {
            let settings = engine.get_resource_mut::<RenderSettings>()
                .ok_or_else(|| mlua::Error::external("render settings are unavailable"))?;
            settings.title = title;
            Ok(())
        })
    })?)?;
    application.set("project_name", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<crate::LuaProjectContext>()
                .map(|context| context.project().manifest().project.name.clone())
                .unwrap_or_else(|| "Vetrace Project".to_owned()))
        })
    })?)?;
    application.set("project_version", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            Ok(engine.get_resource::<crate::LuaProjectContext>()
                .map(|context| context.project().manifest().project.version.clone())
                .unwrap_or_else(|| "0.0.0".to_owned()))
        })
    })?)?;
    env.set("Application", application)?;
    Ok(())
}
