# Lua mods

Simple Shooter loads manifest-driven mods using `vetrace_scripting_lua`. Each
mod is a directory containing `mod.json` and a Lua entry file.

```text
mods/
  my_mod/
    mod.json
    main.lua
```

```json
{
  "id": "my_mod",
  "name": "My Mod",
  "version": "1.0.0",
  "author": "Developer",
  "description": "Example",
  "entry": "main.lua",
  "enabled_by_default": false,
  "capabilities": ["gameplay.rules"],
  "dependencies": [{"id": "shared_rules", "version": "1.0.0"}],
  "conflicts": ["incompatible_rules"],
  "priority": 100
}
```

The script returns a table with optional `on_enable(api)`, `on_disable(api)`,
and `update(api, dt)` callbacks. Mods run in separate Lua states without
`io`, `os`, `package`, `require`, `dofile`, `loadfile`, or raw engine/ECS
handles. A callback error disables that mod and appears in the Mods window.

## API

Read-only context:

- `api:get_number("time")`
- `api:get_number("player_count")`
- `api:get_bool("gameplay_active")`

Approved Simple Shooter commands:

- `api:emit_number("movement_multiplier", value)` (`0.25`–`3.0`)
- `api:emit_number("jump_multiplier", value)` (`0.25`–`3.0`)
- `api:emit_number("gravity_scale", value)` (`0.1`–`3.0`)
- `api:emit_number("vignette_strength", value)` (`0.0`–`1.0`)
- `api:emit_bool("vignette_strength", false)` to release the override
- `api:emit_string("status", message)`
- `api:log(message)`

Enabled states persist in `enabled_mods.json`. Use the in-game Mods window to
select, enable/disable, or reload a mod. Use `--mods-dir PATH` to load another
directory. The bundled `low_gravity` mod demonstrates the full lifecycle.

The generic host only transports typed context and commands. Command meaning
and limits deliberately remain game-side, so other Vetrace games can expose a
different safe API without coupling their rules to the Lua crate.

## Hardening and multiplayer

- Each mod has its own Lua VM, a 16 MiB memory ceiling, a 1 MiB source ceiling,
  and a 500,000-instruction callback budget.
- Undeclared or unavailable capabilities fail closed.
- Dependencies use deterministic priority/id ordering, exact versions, cycle
  detection, and dependent-disable protection. Declared conflicts are checked
  in both directions.
- File changes are detected twice per second and hot reloaded. Values stored
  through `state_get_*`/`state_set_*` survive VM replacement.
- If the mod directory is read-only, enabled-state persistence falls back to
  `.vetrace/mod_state`.
- Multiplayer fingerprints still identify the host's active set for diagnostics,
  but clients do not execute that set or need an identical enabled list. Joined
  clients ignore local gameplay commands and consume the host-authoritative,
  capability-checked aggregate settings. This lets the host enable or disable
  lobby mods without disconnecting clients.
- Multiple gameplay mods compose multiplicatively in deterministic mod-id
  order instead of silently using last-writer-wins behavior.

This substantially limits accidental and hostile Lua behavior inside the
process, but a strict hostile-code security boundary still requires executing
mods in a separate OS process with operating-system sandboxing.
