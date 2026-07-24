use super::*;

mod input;
mod application;
mod ui;
mod scene;
mod physics;
mod audio;
mod assets;
mod events;
mod time;
mod debug;
mod entity;

pub(crate) fn install_game_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    input::install(lua, env)?;
    application::install(lua, env)?;
    ui::install(lua, env)?;
    scene::install(lua, env)?;
    physics::install(lua, env)?;
    audio::install(lua, env)?;
    assets::install(lua, env)?;
    install_modules_api(lua, env)?;
    events::install(lua, env)?;
    time::install(lua, env)?;
    debug::install(lua, env)?;
    entity::install(lua, env)?;
    crate::runtime_api::install_runtime_api(lua, env)?;
    Ok(())
}
