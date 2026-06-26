local server
local cars = {}

function start(engine, self)
    server = engine:create_server('127.0.0.1:4000')
    engine:print('server started')
    engine:spawn_prefab('assets/floor.json')
end

local function handle_message(msg)
    if not msg then return end
    if msg.kind == 'join' then
        local car = engine:spawn_prefab('assets/car.json')
        if car then
            cars[msg.addr] = car
            server:send_custom(msg.addr, 'spawn', '')
            engine:print('spawned car for ' .. msg.addr)
        else
            engine:print('failed to spawn car for ' .. msg.addr)
        end
    elseif msg.kind == 'input' then
        local car = cars[msg.addr]
        if car then
            local f, s = msg.data:match('([^,]+),([^,]+)')
            f = tonumber(f) or 0
            s = tonumber(s) or 0
            car:emit_event('set_input', f, s)
        end
    end
end

function update(engine, self, input, dt)
    server:poll()
    
    while true do
        local msg = server:recv()
        if not msg then break end
        handle_message(msg)
    end
    
    for addr, car in pairs(cars) do
        if car and car.Transform then
            local t = car.Transform
            local state = string.format('%f,%f,%f,%f,%f',
                t.position_x, t.position_y, t.position_z,
                t.orientation_y, t.orientation_w)
            server:send_custom(addr, 'state', state)
        end
    end
end