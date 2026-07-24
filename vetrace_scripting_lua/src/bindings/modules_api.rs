use super::*;

pub(super) fn install_modules_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let modules = lua.create_table()?;
    let cache = lua.create_table()?;
    let loading = lua.create_table()?;
    let module_env = env.clone();
    let require_cache = cache.clone();
    let require_loading = loading.clone();

    let require = lua.create_function(move |lua, path: String| {
        if !path.to_ascii_lowercase().ends_with(".lua") {
            return Err(mlua::Error::external(
                "Modules.require only accepts project-local .lua files",
            ));
        }
        if let Some(value) = require_cache.get::<Option<Value>>(path.as_str())? {
            return Ok(value);
        }
        if require_loading
            .get::<Option<bool>>(path.as_str())?
            .unwrap_or(false)
        {
            return Err(mlua::Error::external(format!(
                "circular Lua module dependency while loading '{path}'",
            )));
        }

        let source = with_context(|engine, _, _, _, _, _| {
            let context = engine
                .get_resource::<crate::LuaProjectContext>()
                .ok_or_else(|| mlua::Error::external("Lua project context is unavailable"))?;
            let resolved = context.resolve_existing(&path).map_err(mlua::Error::external)?;
            std::fs::read_to_string(resolved).map_err(mlua::Error::external)
        })?;

        require_loading.set(path.as_str(), true)?;
        let evaluated = lua
            .load(&source)
            .set_name(format!("@{path}"))
            .set_environment(module_env.clone())
            .eval::<Value>();
        require_loading.set(path.as_str(), Value::Nil)?;

        let value = match evaluated? {
            Value::Nil => Value::Boolean(true),
            value => value,
        };
        require_cache.set(path.as_str(), value.clone())?;
        Ok(value)
    })?;

    modules.set("require", require)?;
    let invalidate_cache = cache.clone();
    modules.set("invalidate", lua.create_function(move |_, path: String| {
        invalidate_cache.set(path.as_str(), Value::Nil)
    })?)?;
    let loaded_cache = cache;
    modules.set("is_loaded", lua.create_function(move |_, path: String| {
        Ok(loaded_cache.get::<Option<Value>>(path.as_str())?.is_some())
    })?)?;
    env.set("Modules", modules)
}
