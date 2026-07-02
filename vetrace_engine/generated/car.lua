local forward, steer = 0, 0
local yaw = 0.0

function start(engine, self)
    self:define_event('set_input')
    self:subscribe_event('set_input', function(f, s)
        forward = f or 0
        steer = s or 0
        engine:print('car received input: forward=' .. tostring(forward) .. ', steer=' .. tostring(steer))
    end)
end

local function quat_from_yaw(y)
    local h = y * 0.5
    return 0, math.sin(h), 0, math.cos(h)
end

function update(engine, self, input, dt)
    local t = self.Transform
    if not t then return end
    
    local speed = 10.0
    local steer_speed = 1.5
    
    yaw = yaw + steer * steer_speed * dt
    local dx = math.sin(yaw) * forward * speed * dt
    local dz = math.cos(yaw) * forward * speed * dt
    
    t.position_x = t.position_x + dx
    t.position_z = t.position_z + dz
    
    local x, y, z, w = quat_from_yaw(yaw)
    t.orientation_x = x
    t.orientation_y = y
    t.orientation_z = z
    t.orientation_w = w
end