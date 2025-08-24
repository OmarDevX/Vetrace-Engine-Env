local player

local function get_player(engine)
    if not player then
        player = engine:find_entity_by_name("Player")
    end
    return player
end

function start(engine, self)
end

function update(engine, self, input, dt)
    local p = get_player(engine)
    if p and p.Transform and self.Transform and self.Velocity then
        local dx = p.Transform.position_x - self.Transform.position_x
        local dy = p.Transform.position_y - self.Transform.position_y
        local dist = math.sqrt(dx*dx + dy*dy)
        if dist > 0 then
            self.Velocity.velocity_x = dx / dist * 2.0
            self.Velocity.velocity_y = dy / dist * 2.0
        end
    end
end