return {
    ready = function(self)
        Debug.log("Lua Gameplay Runtime v1 started")
        Debug.log("Move the orange cube with A/D or Left/Right arrows")
        Debug.log("The purple cube uses the same Lua script but keeps separate state")
        self.spawned_probe = false
    end,

    update = function(self, _dt)
        if not self.spawned_probe then
            local probe = Scene.spawn("Deferred Lua Spawn Probe")
            probe:add_tag("spawned_from_lua")
            probe:set_translation(0.0, -20.0, 0.0)
            self.spawned_probe = true
            Debug.log("Deferred Scene.spawn command completed after the callback")
        end
    end,
}
