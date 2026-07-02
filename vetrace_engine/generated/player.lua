mouse_down = false

function start(engine, self)
    player = self
end

local function forward_from_orientation(o)
    local angle = 2 * math.atan2(o.orientation_z, o.orientation_w)
    return math.cos(angle), math.sin(angle)
end

function update(engine, self, input, dt)
    local t = self.Transform
    if t then
        local dx, dy = 0, 0
        if input:is_key_down("W") then dy = dy + 1 end
        if input:is_key_down("S") then dy = dy - 1 end
        if input:is_key_down("D") then dx = dx + 1 end
        if input:is_key_down("A") then dx = dx - 1 end
        local len = math.sqrt(dx*dx + dy*dy)
        if len > 0 then dx = dx/len; dy = dy/len end
        local speed = 3.0
        t.position_x = t.position_x + dx * dt * speed
        t.position_y = t.position_y + dy * dt * speed
    end
    local pressed = input:is_mouse_button_down("Left")
    if pressed and not mouse_down then
        local bullet = engine:spawn_prefab("assets/bullet.json")
        if bullet then
            if bullet.Transform and t then
                bullet.Transform.position_x = t.position_x
                bullet.Transform.position_y = t.position_y
                bullet.Transform.position_z = t.position_z
                bullet.Transform.orientation_x = t.orientation_x
                bullet.Transform.orientation_y = t.orientation_y
                bullet.Transform.orientation_z = t.orientation_z
                bullet.Transform.orientation_w = t.orientation_w
                local fx, fy = forward_from_orientation(bullet.Transform)
                if bullet.Velocity then
                    bullet.Velocity.velocity_x = fx * 15
                    bullet.Velocity.velocity_y = fy * 15
                    bullet.Velocity.velocity_z = 0
                end
            end
        end
    end
    mouse_down = pressed
end

function on_collision(engine, self, other)
    if other:has_tag("enemy") then
        engine:clear_scene()
        engine:spawn_prefab("assets/player.json")
        engine:spawn_prefab("assets/game_controller.json")
        engine:spawn_prefab("assets/score_label.json")
    end
end