use super::*;

pub(super) fn install_storage_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let storage = lua.create_table()?;
    storage.set("exists", lua.create_function(|_, path: String| {
        with_context(|engine, _, _, _, _, _| Ok(storage_path(engine, &path)?.is_file()))
    })?)?;
    storage.set("read_text", lua.create_function(|_, path: String| {
        with_context(|engine, _, _, _, _, _| {
            let path = storage_path(engine, &path)?;
            if !path.is_file() { return Ok(None); }
            fs::read_to_string(path).map(Some).map_err(mlua::Error::external)
        })
    })?)?;
    storage.set("write_text", lua.create_function(|_, (path, contents): (String, String)| {
        with_context(|engine, _, _, _, _, _| atomic_write(&storage_path(engine, &path)?, contents.as_bytes()))
    })?)?;
    storage.set("read_json", lua.create_function(|lua, path: String| {
        with_context(|engine, _, _, _, _, _| {
            let path = storage_path(engine, &path)?;
            if !path.is_file() { return Ok(Value::Nil); }
            let text = fs::read_to_string(path).map_err(mlua::Error::external)?;
            let value: JsonValue = serde_json::from_str(&text).map_err(mlua::Error::external)?;
            json_to_lua(lua, &value, 0)
        })
    })?)?;
    storage.set("write_json", lua.create_function(|_, (path, value): (String, Value)| {
        with_context(|engine, _, _, _, _, _| {
            let value = lua_to_json(value, 0)?;
            let bytes = serde_json::to_vec_pretty(&value).map_err(mlua::Error::external)?;
            atomic_write(&storage_path(engine, &path)?, &bytes)
        })
    })?)?;
    storage.set("remove", lua.create_function(|_, path: String| {
        with_context(|engine, _, _, _, _, _| {
            let path = storage_path(engine, &path)?;
            if path.exists() { fs::remove_file(path).map_err(mlua::Error::external)?; }
            Ok(())
        })
    })?)?;
    env.set("Storage", storage)
}

pub(super) fn storage_path(engine: &vetrace_core::Engine, raw: &str) -> mlua::Result<PathBuf> {
    let relative = ProjectPath::new(raw).map_err(mlua::Error::external)?;
    let context = engine.get_resource::<crate::LuaProjectContext>().ok_or_else(|| mlua::Error::external("Lua project context is unavailable"))?;
    Ok(context.project().root().join(".vetrace/user").join(relative.as_path()))
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> mlua::Result<()> {
    let parent = path.parent().ok_or_else(|| mlua::Error::external("storage path has no parent"))?;
    fs::create_dir_all(parent).map_err(mlua::Error::external)?;
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(mlua::Error::external)?;
    file.write_all(bytes).map_err(mlua::Error::external)?;
    file.sync_all().map_err(mlua::Error::external)?;
    drop(file);
    #[cfg(windows)]
    if path.exists() { fs::remove_file(path).map_err(mlua::Error::external)?; }
    fs::rename(temporary, path).map_err(mlua::Error::external)
}
