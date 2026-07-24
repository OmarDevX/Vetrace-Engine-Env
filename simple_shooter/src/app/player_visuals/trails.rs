use super::*;

pub(crate) fn safe_normalize(v: Vec3, fallback: Vec3) -> Vec3 {
    if v.length_squared() > 1.0e-8 {
        v.normalize()
    } else {
        fallback
    }
}

pub(crate) fn spawn_bullet_trail(engine: &mut Engine, weapon_id: &str, from: Vec3, to: Vec3) {
    let config = weapon_definition(engine, weapon_id);
    if !config.tracer.enabled { return; }
    let center = (from + to) * 0.5;
    let length = from.distance(to).max(0.1);
    let delta = to - from;
    let dir = safe_normalize(delta, Vec3::Z);
    let rotation = Quat::from_rotation_arc(Vec3::Z, dir);
    engine
        .spawn_actor("Bullet Trail")
        .with(Transform { translation: center, rotation, scale: Vec3::ONE })
        .with(Shape { primitive: PrimitiveShape::Cube, size: Vec3::new(config.tracer.width, config.tracer.width, length) })
        .with(Material { base_color: vec3(config.tracer.color), emissive: vec3(config.tracer.emissive), ..Material::default() })
        .with(EmissiveLightEmitter {
            intensity: config.tracer.light_intensity,
            range: config.tracer.light_range,
            local_axis: Vec3::Z,
            length,
            samples: config.tracer.light_samples,
            ..EmissiveLightEmitter::default()
        })
        .with(Renderable { visible: true, ..Renderable::default() })
        .with(BulletTrail { ttl: config.tracer.lifetime_seconds, from, to })
        .build();
}

pub(crate) fn update_bullet_trails(engine: &mut Engine, dt: f32) {
    let mut expired = Vec::new();
    engine.query_mut::<BulletTrail>().for_each(|actor, trail| {
        trail.ttl -= dt;
        if trail.ttl <= 0.0 {
            expired.push(actor);
        }
    });
    engine.defer(|commands| {
        for actor in expired {
            commands.despawn(actor);
        }
    });
}

pub(crate) fn update_player_shader_params(engine: &mut Engine, time: f32) {
    let players: Vec<(Actor, f32, f32)> = engine.actors_with::<ShooterPlayer>()
        .into_iter()
        .map(|(actor, player)| {
            let y = actor.get_component::<Transform>(engine).map(|t| t.translation.y).unwrap_or(0.0);
            let health01 = player.health as f32 / MAX_HEALTH as f32;
            (actor, y, health01)
        })
        .collect();

    for (actor, _y, health01) in players {
        let Some((seed, color_a, color_b)) = actor.get_component_mut::<PlayerGradientShader>(engine).map(|shader| {
            shader.time = time;
            (shader.seed, shader.color_a, shader.color_b)
        }) else { continue; };
        if let Some(custom) = actor.get_component_mut::<CustomShaderMaterial>(engine) {
            if custom.params.len() < 2 {
                custom.params.resize(2, 0.0);
            }
            custom.params[0] = seed;
            custom.params[1] = health01;
            custom.fallback_color_a = color_a;
            custom.fallback_color_b = color_b;
        }
    }
}
