return {
    properties = {
        speed = { type = "number", default = 4.0 },
        controlled = { type = "boolean", default = false },
        direction = { type = "number", default = 1.0 },
        min_x = { type = "number", default = -3.0 },
        max_x = { type = "number", default = 3.0 },
        label = { type = "string", default = "mover" },
    },

    ready = function(self)
        self.entity:add_tag("lua_mover")
        self.distance_travelled = 0.0
        self.fixed_ticks = 0
        Debug.log(self.label .. " ready with isolated instance state")
    end,

    update = function(self, dt)
        local movement = 0.0
        if self.controlled then
            if Input.action_down("move_left") then
                movement = movement - 1.0
            end
            if Input.action_down("move_right") then
                movement = movement + 1.0
            end
        else
            local transform = self.components.Transform
            if transform.translation.x >= self.max_x then
                self.direction = -1.0
            elseif transform.translation.x <= self.min_x then
                self.direction = 1.0
            end
            movement = self.direction
        end

        local delta = movement * self.speed * dt
        local transform = self.components.Transform
        transform.translation.x = transform.translation.x + delta
        self.distance_travelled = self.distance_travelled + math.abs(delta)
    end,

    fixed_update = function(self, _dt)
        self.fixed_ticks = self.fixed_ticks + 1
    end,

    destroy = function(self)
        Debug.log(self.label .. " destroyed after " .. tostring(self.fixed_ticks) .. " fixed ticks")
    end,
}
