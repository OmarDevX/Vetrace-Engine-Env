use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use mlua::{Lua, Table, Value};
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use vetrace_net::UdpChannel;
use vetrace_project::ProjectPath;
use vetrace_render::{
    AmbientOcclusionMode, AntiAliasingMode, Camera, PresentModePreference, RenderSettings,
    ShadowFilterMode,
};

use crate::context::{queue_command, with_context, LuaCommand};

mod camera;
mod json;
mod network;
mod rendering;
mod storage;
mod value_helpers;
mod window;

use camera::install_camera_api;
use json::{install_json_api, json_to_lua, lua_to_json};
use network::{install_network_api, validate_channel_name, with_network_channel_mut};
use rendering::install_rendering_api;
use storage::{atomic_write, install_storage_api, storage_path};
use value_helpers::{expect_bool, expect_number, expect_string};
use window::install_window_api;


const MAX_JSON_DEPTH: usize = 32;
const MAX_NETWORK_PACKET_SIZE: usize = 60 * 1024;

#[derive(Debug, Default)]
pub struct LuaNetworkState {
    channels: HashMap<String, UdpChannel>,
}

#[derive(Clone, Copy, Debug)]
pub struct LuaAudioSettings {
    pub master_volume: f32,
}

impl Default for LuaAudioSettings {
    fn default() -> Self { Self { master_volume: 1.0 } }
}

pub(crate) fn install_runtime_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    install_window_api(lua, env)?;
    install_rendering_api(lua, env)?;
    install_camera_api(lua, env)?;
    install_storage_api(lua, env)?;
    install_json_api(lua, env)?;
    install_network_api(lua, env)?;
    Ok(())
}

pub(crate) fn master_volume() -> mlua::Result<f32> {
    with_context(|engine, _, _, _, _, _| {
        Ok(engine
            .get_resource::<LuaAudioSettings>()
            .copied()
            .unwrap_or_default()
            .master_volume
            .clamp(0.0, 2.0))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_names_are_sandboxed_identifiers() {
        assert!(validate_channel_name("discovery_1").is_ok());
        assert!(validate_channel_name("../socket").is_err());
        assert!(validate_channel_name("").is_err());
    }

    #[test]
    fn network_payload_limit_stays_below_udp_ceiling() {
        assert!(MAX_NETWORK_PACKET_SIZE < 65_507);
    }
}
