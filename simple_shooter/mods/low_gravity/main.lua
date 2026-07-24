local mod = {}

function mod.on_enable(api)
    api:emit_number("movement_multiplier", 1.25)
    api:emit_number("jump_multiplier", 1.45)
    api:emit_number("gravity_scale", 0.55)
    api:emit_string("status", "Low Gravity Arena rules activated")
    api:log("enabled")
end

function mod.on_disable(api)
    api:emit_number("movement_multiplier", 1.0)
    api:emit_number("jump_multiplier", 1.0)
    api:emit_number("gravity_scale", 1.0)
    api:emit_string("status", "Standard arena rules restored")
    api:log("disabled")
end

function mod.update(api, dt)
    -- Context is read-only. This example keeps its effects event-driven, but a
    -- mod may inspect values such as time, player_count, and gameplay_active.
    local active = api:get_bool("gameplay_active")
    if active and dt > 1.0 then
        api:log("unusually long frame detected")
    end
end

return mod
