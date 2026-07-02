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
    if self.Lifetime then
        self.Lifetime.remaining = self.Lifetime.remaining - dt
        if self.Lifetime.remaining <= 0 then
            engine:delete_entity(self)
        end
    end
end

function on_collision(engine, self, other)
    if other:has_tag("enemy") then
        local points = 1
        if other.ScoreValue then
            points = other.ScoreValue.value
        end
        engine:delete_entity(other)
        engine:delete_entity(self)
        local p = get_player(engine)
        if p and p.Score then
            p.Score.value = p.Score.value + points
        end
        engine:request_redraw()
    end
end