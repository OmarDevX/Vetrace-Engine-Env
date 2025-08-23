spawn_timer = 0
score_label = nil
player = nil

local function spawn_enemy(engine)
    local enemy = engine:spawn_prefab("assets/enemy.json")
    if enemy and enemy.Transform then
        enemy.Transform.position_x = math.random() * 20 - 10
        enemy.Transform.position_y = math.random() * 20 - 10
        enemy.Transform.position_z = 0
    end
end

function start(engine, self)
    math.randomseed(os.time())
    player = engine:find_entity_by_name("Player")
    if player and player.Score then
        player.Score.value = 0
    end
    score_label = engine:spawn_prefab("assets/score_label.json")
    spawn_enemy(engine)
end

function update(engine, self, input, dt)
    if not player then
        player = engine:find_entity_by_name("Player")
    end
    spawn_timer = spawn_timer + dt
    if spawn_timer >= 2.0 then
        spawn_timer = 0
        spawn_enemy(engine)
    end
    if score_label and score_label.UILabel and player and player.Score then
        local s = player.Score.value
        score_label.UILabel.text = "Score: " .. tostring(s)
        engine:request_redraw()
    end
end