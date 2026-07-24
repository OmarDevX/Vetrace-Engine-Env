use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let audio = lua.create_table()?;
    audio.set("play", lua.create_function(|lua, (path, volume): (String, Option<f32>)| {
        queue_audio(lua, path, None, volume.unwrap_or(1.0), false)
    })?)?;
    audio.set("play_3d", lua.create_function(
        |lua, (path, x_or_vector, y, z, volume): (String, Value, Option<f32>, Option<f32>, Option<f32>)| {
            let position = parse_vec3_argument(x_or_vector, y, z, "Audio.play_3d")?;
            queue_audio(lua, path, Some(position), volume.unwrap_or(1.0), false)
        },
    )?)?;
    audio.set("play_loop", lua.create_function(|lua, (path, volume): (String, Option<f32>)| {
        queue_audio(lua, path, None, volume.unwrap_or(1.0), true)
    })?)?;
    audio.set("master_volume", lua.create_function(|_, ()| crate::runtime_api::master_volume())?)?;
    audio.set("set_master_volume", lua.create_function(|_, volume: f32| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(settings) = engine.get_resource_mut::<crate::runtime_api::LuaAudioSettings>() {
                settings.master_volume = volume.clamp(0.0, 2.0);
            }
            Ok(())
        })
    })?)?;
    env.set("Audio", audio)?;
    Ok(())
}
