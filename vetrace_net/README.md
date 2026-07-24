# vetrace_net game drivers

`GameServerDriver` and `GameClientDriver` are the short path for multiplayer
games. They use `GameNetPacket`, an engine-owned envelope whose generic payloads
remain fully game-defined:

- `Input`: high-rate sequenced player input
- `Rpc`: named gameplay calls with delivery configuration
- `State`: persistent snapshot state
- `Event`: transient replicated effects such as shots or sounds
- `Message`: game-only lobby, chat, or content-transfer messages
- `Hello` / `Welcome`: game-specific join data

The drivers own hello retry, compatibility validation, authenticated client
identity, input acknowledgements, RPC acknowledgements/resends, snapshot
framing, replicated-event draining, leave handling, and timeout tracking.

```rust
type Packet = GameNetPacket<Input, Rpc, PlayerState, ShotFx, Message, Hello, Welcome>;
type Server = GameServerDriver<Input, Rpc, PlayerState, ShotFx, Message, Hello, Welcome, ClientData>;

let compatibility = CompatibilityManifest::new(PROTOCOL_VERSION)
    .with_gameplay_hash("weapons", weapon_definitions_hash);
let mut server = Server::new(channel, FIRST_CLIENT_ID, 30.0, compatibility);
server.register_rpc::<Rpc>("fire_weapon").any_peer().call_remote().unreliable_ordered();

for event in server.poll(dt) {
    match event {
        ServerGameEvent::JoinRequested { addr, payload } => {
            server.accept_with(addr, |id| spawn_client(id, payload), Welcome::default());
        }
        ServerGameEvent::Input { client_id, input, .. } => apply_input(client_id, input),
        ServerGameEvent::Message { client_id, message, .. } => handle_message(client_id, message),
        ServerGameEvent::DisconnectRequested { addr } => remove_client(addr),
    }
}

server.queue_events(shots_this_tick);
if server.should_send_snapshot(dt, server.has_pending_events()) {
    server.flush_snapshot(player_states);
}
```

Games that need a custom wire protocol can continue using the lower-level
`MultiplayerServer`, `MultiplayerClient`, sessions, protocol primitives, and
`TypedUdpChannel` directly.
