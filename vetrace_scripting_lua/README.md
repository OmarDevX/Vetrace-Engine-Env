# Vetrace Lua Gameplay Runtime

`vetrace_scripting_lua` provides trusted project gameplay scripting. The
separate `modding` module remains the restricted API for third-party mods.

## Entity script

```lua
return {
    properties = {
        speed = { type = "number", default = 5.0 },
        title = { type = "string", default = "Player" },
    },

    ready = function(self)
        self.distance = 0.0
    end,

    update = function(self, dt)
        local direction = 0.0
        if Input.action_down("move_left") then direction = direction - 1.0 end
        if Input.action_down("move_right") then direction = direction + 1.0 end
        local movement = direction * self.speed * dt
        self.transform:translate_xyz(movement, 0.0, 0.0)
        self.distance = self.distance + math.abs(movement)
    end,

    fixed_update = function(self, dt)
    end,

    on_event = function(self, name, payload)
    end,

    on_collision_enter = function(self, other)
    end,

    on_collision_exit = function(self, other)
    end,

    destroy = function(self)
    end,
}
```

Every entity receives a separate `self` table. Fields created at runtime are
not shared with another entity using the same script template.

## Scene property overrides

```json
{
  "type": "vetrace.scripting.lua_script",
  "data": {
    "script": "assets/scripts/player.lua",
    "enabled": true,
    "properties": {
      "speed": 8.0,
      "title": "Fast Player"
    }
  }
}
```

## Built-in modules

- `Input`: keys, mouse state, and project input actions.
- `Scene`: deferred spawn, destroy, clear, tag/name lookup, and entity count.
- `Modules`: cached project-local Lua modules.
- `Events`: targeted and broadcast gameplay messages.
- `Assets`, `Storage`, and `Json`: sandboxed project assets and user data.
- `UI`, `Rendering`, `Audio`, `Physics`, and `Camera`: generic runtime services.
  `Physics.set_enabled(entity, enabled)` cleanly activates or deactivates both
  the runtime rigid body and collider when present.
- `Time`: callback delta and fixed-update state.
- `Entity`: current callback entity lookup.
- `Debug`: log, warning, and error output.

`self.entity` is a safe generational entity handle and `self.transform` exposes
transform access. These handles contain no permanent engine pointer. Engine
access exists only for the duration of a callback.

Structural scene operations are deferred until the callback returns, avoiding
ECS mutation while script iteration is active. A pending entity returned by
`Scene.spawn` can be configured during the same callback.

## Legacy scripts

Scripts using the earlier callbacks continue to work:

```lua
return {
    start = function(engine, entity) end,
    update = function(engine, entity, input, dt) end,
}
```

New projects should use the gameplay lifecycle.

## Project Lua modules

Reusable trusted-project code can be split into normal Lua modules under
`assets/`. Load modules from a running lifecycle callback:

```lua
ready = function(self)
    self.ui = Modules.require("assets/scripts/ui.lua")
end
```

`Modules.require(path)`:

- accepts only project-local `.lua` files resolved through the project sandbox;
- evaluates the module in the requiring script's API environment;
- caches the returned value for that script environment;
- reports circular dependencies;
- treats a module returning `nil` as `true`, matching normal Lua `require` behavior.

`Modules.invalidate(path)` clears one cached module and
`Modules.is_loaded(path)` reports whether it is cached. The development hot
reload plugin restarts active entry scripts when a helper module changes, so
module caches are rebuilt without restarting the player.

Module loading is callback-scoped because project filesystem access is only
available while a trusted gameplay callback is running. Load modules in
`ready`, then keep them on `self` for later callbacks.

## Targeted gameplay events

Use targeted events for one-off gameplay messages instead of temporary tags:

```lua
Events.emit(target, "damage", { amount = 20.0 })

on_event = function(self, name, payload)
    if name == "damage" then
        self.health = self.health - payload.amount
    end
end
```

Tags remain appropriate for durable classification and state such as `player`,
`enemy`, `paused`, or `inactive`.
