# Lua Gameplay Runtime Example

This is a Rust-free Vetrace project. The manifest, scene, exported script properties, and gameplay are project data plus Lua.

Run through the installed-style generic player:

```bash
cargo run -p vetrace_player -- examples/lua_runtime_project
```

Expected behavior:

- The orange cube is controlled with A/D or Left/Right.
- The purple cube moves automatically.
- Both cubes use the same `player.lua` template with separate instance tables and property overrides.
- `ready`, `update`, `fixed_update`, and `destroy` use the gameplay lifecycle.
- The autoload queues a deferred `Scene.spawn` command.
- No game-specific Rust crate is involved.
