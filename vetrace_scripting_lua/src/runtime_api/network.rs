use super::*;

pub(super) fn install_network_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let net = lua.create_table()?;
    net.set("open", lua.create_function(|_, (name, bind_addr): (String, String)| {
        validate_channel_name(&name)?;
        with_context(|engine, _, _, _, _, _| {
            let channel = UdpChannel::bind(&bind_addr).map_err(mlua::Error::external)?;
            let state = engine
                .get_resource_mut::<LuaNetworkState>()
                .ok_or_else(|| mlua::Error::external("Lua networking is unavailable"))?;
            state.channels.insert(name, channel);
            Ok(())
        })
    })?)?;
    net.set("close", lua.create_function(|_, name: String| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(state) = engine.get_resource_mut::<LuaNetworkState>() {
                state.channels.remove(&name);
            }
            Ok(())
        })
    })?)?;
    net.set("is_open", lua.create_function(|_, name: String| {
        with_context(|engine, _, _, _, _, _| Ok(engine.get_resource::<LuaNetworkState>().is_some_and(|state| state.channels.contains_key(&name))))
    })?)?;
    net.set("local_addr", lua.create_function(|_, name: String| {
        with_context(|engine, _, _, _, _, _| {
            let state = engine.get_resource::<LuaNetworkState>().ok_or_else(|| mlua::Error::external("Lua networking is unavailable"))?;
            let channel = state.channels.get(&name).ok_or_else(|| mlua::Error::external(format!("network channel '{name}' is not open")))?;
            channel.local_addr().map(|addr| addr.to_string()).map_err(mlua::Error::external)
        })
    })?)?;
    net.set("set_broadcast", lua.create_function(|_, (name, enabled): (String, bool)| {
        with_network_channel_mut(&name, |channel| channel.set_broadcast(enabled).map_err(mlua::Error::external))
    })?)?;
    net.set("add_peer", lua.create_function(|_, (name, addr): (String, String)| {
        let addr = parse_socket_addr(&addr)?;
        with_network_channel_mut(&name, |channel| { channel.add_peer(addr); Ok(()) })
    })?)?;
    net.set("remove_peer", lua.create_function(|_, (name, addr): (String, String)| {
        let addr = parse_socket_addr(&addr)?;
        with_network_channel_mut(&name, |channel| { channel.remove_peer(addr); Ok(()) })
    })?)?;
    net.set("peers", lua.create_function(|lua, name: String| {
        with_context(|engine, _, _, _, _, _| {
            let state = engine.get_resource::<LuaNetworkState>().ok_or_else(|| mlua::Error::external("Lua networking is unavailable"))?;
            let channel = state.channels.get(&name).ok_or_else(|| mlua::Error::external(format!("network channel '{name}' is not open")))?;
            let table = lua.create_table()?;
            for (index, peer) in channel.peers().iter().enumerate() { table.set(index + 1, peer.to_string())?; }
            Ok(table)
        })
    })?)?;
    net.set("send", lua.create_function(|_, (name, addr, packet): (String, String, Value)| {
        let addr = parse_socket_addr(&addr)?;
        let bytes = encode_network_packet(packet)?;
        with_network_channel_mut(&name, |channel| channel.send_to(addr, &bytes).map(|_| ()).map_err(mlua::Error::external))
    })?)?;
    net.set("broadcast", lua.create_function(|_, (name, packet): (String, Value)| {
        let bytes = encode_network_packet(packet)?;
        with_network_channel_mut(&name, |channel| { channel.broadcast(&bytes); Ok(()) })
    })?)?;
    net.set("poll", lua.create_function(|lua, (name, max_packets): (String, Option<usize>)| {
        with_context(|engine, _, _, _, _, _| {
            let state = engine.get_resource_mut::<LuaNetworkState>().ok_or_else(|| mlua::Error::external("Lua networking is unavailable"))?;
            let channel = state.channels.get_mut(&name).ok_or_else(|| mlua::Error::external(format!("network channel '{name}' is not open")))?;
            let packets = channel.recv_up_to(MAX_NETWORK_PACKET_SIZE, max_packets.unwrap_or(128).clamp(1, 2048));
            let out = lua.create_table()?;
            for (index, (addr, bytes)) in packets.into_iter().enumerate() {
                let packet = lua.create_table()?;
                packet.set("addr", addr.to_string())?;
                match serde_json::from_slice::<JsonValue>(&bytes) {
                    Ok(value) => packet.set("data", json_to_lua(lua, &value, 0)?)?,
                    Err(error) => packet.set("error", error.to_string())?,
                }
                out.set(index + 1, packet)?;
            }
            Ok(out)
        })
    })?)?;
    env.set("Net", net)
}

pub(super) fn with_network_channel_mut<R>(name: &str, operation: impl FnOnce(&mut UdpChannel) -> mlua::Result<R>) -> mlua::Result<R> {
    with_context(|engine, _, _, _, _, _| {
        let state = engine.get_resource_mut::<LuaNetworkState>().ok_or_else(|| mlua::Error::external("Lua networking is unavailable"))?;
        let channel = state.channels.get_mut(name).ok_or_else(|| mlua::Error::external(format!("network channel '{name}' is not open")))?;
        operation(channel)
    })
}

pub(super) fn encode_network_packet(value: Value) -> mlua::Result<Vec<u8>> {
    let json = lua_to_json(value, 0)?;
    let bytes = serde_json::to_vec(&json).map_err(mlua::Error::external)?;
    if bytes.len() > MAX_NETWORK_PACKET_SIZE {
        return Err(mlua::Error::external(format!("network packet is {} bytes; maximum is {MAX_NETWORK_PACKET_SIZE}", bytes.len())));
    }
    Ok(bytes)
}

pub(super) fn validate_channel_name(name: &str) -> mlua::Result<()> {
    if name.is_empty() || name.len() > 64 || !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')) {
        return Err(mlua::Error::external("network channel names must be 1-64 ASCII letters, numbers, '_' or '-'"));
    }
    Ok(())
}

pub(super) fn parse_socket_addr(raw: &str) -> mlua::Result<SocketAddr> {
    raw.parse().map_err(|error| mlua::Error::external(format!("invalid socket address '{raw}': {error}")))
}
