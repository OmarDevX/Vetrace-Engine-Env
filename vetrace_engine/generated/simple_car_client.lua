local client
local car

function start(engine, self)
    client = engine:create_client('127.0.0.1:4000')
    client:send_ping()
    client:send_custom('join', '')
    engine:spawn_prefab('assets/floor.json')
end

local function handle_message(msg)
    if not msg then return end
    if msg.kind == 'spawn' then
        car = engine:spawn_prefab('assets/car.json')
    elseif msg.kind == 'state' then
        if car and car.Transform then
            local px, py, pz, oy, ow = msg.data:match('([^,]+),([^,]+),([^,]+),([^,]+),([^,]+)')
            car.Transform.position_x = tonumber(px) or 0
            car.Transform.position_y = tonumber(py) or 0
            car.Transform.position_z = tonumber(pz) or 0
            car.Transform.orientation_x = 0
            car.Transform.orientation_y = tonumber(oy) or 0
            car.Transform.orientation_z = 0
            car.Transform.orientation_w = tonumber(ow) or 1
        end
    end
end

function update(engine, self, input, dt)
    local f, s = 0, 0
    if input:is_key_down('W') then f = 1 end
    if input:is_key_down('S') then f = -1 end
    if input:is_key_down('A') then s = 1 end
    if input:is_key_down('D') then s = -1 end
    
    client:send_custom('input', string.format('%f,%f', f, s))
    client:poll()
    
    while true do
        local msg = client:recv()
        if not msg then break end
        handle_message(msg)
    end
end