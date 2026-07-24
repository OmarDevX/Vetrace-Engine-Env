pub(crate) fn player_body_rotation(yaw: f32) -> Quat {
    // The shooter camera convention is yaw=0 looking down -Z, with positive
    // yaw turning the view/right-hand ray toward +X. A mesh/model whose forward
    // side is -Z must therefore use -yaw for its world rotation. Using +yaw
    // made remote bodies appear to rotate opposite the camera/shooting ray.
    Quat::from_rotation_y(-yaw)
}

use super::*;

pub(crate) fn crosshair_aim_point(
    engine: &Engine,
    shooter: Actor,
    shooter_id: u64,
    origin: Vec3,
    dir: Vec3,
    range: f32,
) -> Vec3 {
    let dir = safe_normalize(dir, Vec3::NEG_Z);
    let mut best_player: Option<(f32, Vec3)> = None;
    for (actor, player) in engine.actors_with::<ShooterPlayer>() {
        if actor == shooter || player.id == shooter_id || !player.alive { continue; }
        let Some(transform) = actor.get_component::<Transform>(engine) else { continue; };
        let center = player_eye_position(transform.translation);
        let distance = (center - origin).dot(dir);
        if !(0.0..=range).contains(&distance) { continue; }
        let closest = origin + dir * distance;
        if center.distance_squared(closest) > (PLAYER_RADIUS * 1.4).powi(2) { continue; }
        if best_player.as_ref().map(|(best, _)| distance < *best).unwrap_or(true) {
            best_player = Some((distance, closest));
        }
    }

    let blocker = first_hitscan_blocker(engine, shooter, origin, dir, range);
    match (best_player, blocker) {
        (Some((player_distance, _point)), Some(hit)) if hit.distance + HITSCAN_SURFACE_EPSILON < player_distance => hit.position,
        (Some((_, point)), _) => point,
        (None, Some(hit)) => hit.position,
        (None, None) => origin + dir * range,
    }
}

pub(crate) fn resolve_fire_request(engine: &mut Engine, request: FireRequest) -> Option<ShotResult> {
    if !request.shooter.is_alive(engine) { return None; }
    let weapon = weapon_definition(engine, &request.weapon_id);
    let (muzzle, barrel_forward) = weapon_muzzle(engine, request.shooter, &weapon)
        .unwrap_or((request.aim_origin, request.aim_direction));
    let aim_point = match weapon.gameplay.aim_mode {
        WeaponAimMode::CrosshairConverge => {
            Some(crosshair_aim_point(
                engine,
                request.shooter,
                request.shooter_id,
                request.aim_origin,
                request.aim_direction,
                weapon.gameplay.range,
            ))
        }
        WeaponAimMode::BarrelForward => None,
    };
    let direction = physical_shot_direction(
        weapon.gameplay.aim_mode,
        muzzle,
        barrel_forward,
        aim_point,
        request.aim_direction,
    );
    fire_hitscan(
        engine,
        request.shooter,
        request.shooter_id,
        &request.weapon_id,
        muzzle,
        direction,
        weapon.gameplay.range,
        weapon.gameplay.damage,
    )
}

pub(crate) fn physical_shot_direction(
    aim_mode: WeaponAimMode,
    muzzle: Vec3,
    barrel_forward: Vec3,
    aim_point: Option<Vec3>,
    fallback: Vec3,
) -> Vec3 {
    match aim_mode {
        WeaponAimMode::CrosshairConverge => safe_normalize(aim_point.unwrap_or(muzzle + fallback) - muzzle, fallback),
        WeaponAimMode::BarrelForward => safe_normalize(barrel_forward, fallback),
    }
}

pub(crate) fn fire_hitscan(
    engine: &mut Engine,
    shooter: Actor,
    shooter_id: u64,
    weapon_id: &str,
    origin: Vec3,
    dir: Vec3,
    range: f32,
    damage: i32,
 ) -> Option<ShotResult> {
    if let Some(stats) = engine.get_resource_mut::<ShooterStats>() {
        stats.shots_fired = stats.shots_fired.saturating_add(1);
    }

    let dir = dir.normalize_or_zero();
    if dir.length_squared() == 0.0 { return None; }

    let candidates: Vec<(Actor, u64, Vec3)> = engine.actors_with::<ShooterPlayer>()
        .into_iter()
        .filter_map(|(actor, player)| {
            if player.id == shooter_id || !player.alive { return None; }
            let pos = player_eye_position(actor.get_component::<Transform>(engine)?.translation);
            Some((actor, player.id, pos))
        })
        .collect();

    let mut best: Option<(Actor, u64, f32, Vec3)> = None;
    for (candidate, id, center) in candidates {
        let to_center = center - origin;
        let t = to_center.dot(dir);
        if !(0.0..=range).contains(&t) { continue; }
        let closest = origin + dir * t;
        let dist_sq = center.distance_squared(closest);
        if dist_sq <= (PLAYER_RADIUS * 1.4).powi(2) {
            match best {
                Some((_, _, best_t, _)) if t >= best_t => {}
                _ => best = Some((candidate, id, t, closest)),
            }
        }
    }

    let blocker = first_hitscan_blocker(engine, shooter, origin, dir, range);
    let mut end = origin + dir * range;
    let mut hit_id = None;

    if let Some((target, target_id, distance, hit_point)) = best {
        let blocked = blocker
            .as_ref()
            .map(|hit| hit.distance + HITSCAN_SURFACE_EPSILON < distance)
            .unwrap_or(false);
        if blocked {
            if let Some(hit) = blocker {
                end = hit.position;
            }
        } else {
            end = hit_point;
            hit_id = Some(target_id);
            let killer_name = shooter.get_component::<ShooterPlayer>(engine)
                .map(|player| player.name.clone())
                .unwrap_or_else(|| format!("Player {shooter_id}"));
            let killed = damage_player(engine, target, damage, shooter_id, &killer_name);
            if killed {
                let shooter_actor = engine.actors_with::<ShooterPlayer>().into_iter().find(|(_, player)| player.id == shooter_id).map(|(actor, _)| actor);
                if let Some(shooter_actor) = shooter_actor {
                    if let Some(player) = shooter_actor.get_component_mut::<ShooterPlayer>(engine) { player.kills = player.kills.saturating_add(1); }
                }
            }
            if let Some(stats) = engine.get_resource_mut::<ShooterStats>() {
                stats.hits = stats.hits.saturating_add(1);
            }
            println!("player {shooter_id} hit player {target_id} at {distance:.1}m");
        }
    } else if let Some(hit) = blocker {
        end = hit.position;
    }

    Some(ShotResult {
        shooter_id,
        weapon_id: weapon_id.to_string(),
        muzzle: origin,
        endpoint: end,
        hit_id,
    })
}

pub(crate) fn process_fire_requests(engine: &mut Engine) -> Vec<ShotSnapshot> {
    let requests = engine.drain_events::<FireRequest>();
    let mut snapshots = Vec::with_capacity(requests.len());
    for request in requests {
        if let Some(result) = resolve_fire_request(engine, request) {
            snapshots.push(ShotSnapshot {
                weapon_id: result.weapon_id.clone(),
                from: v3(result.muzzle),
                to: v3(result.endpoint),
                shooter_id: result.shooter_id,
                hit_id: result.hit_id,
            });
            engine.send_event(result);
        }
    }
    snapshots
}


pub(crate) const HITSCAN_SURFACE_EPSILON: f32 = 0.035;

pub(crate) fn first_hitscan_blocker(
    engine: &Engine,
    shooter: Actor,
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
) -> Option<vetrace_core::RaycastHit> {
    vetrace_physics::raycast_colliders(engine, origin, dir, max_distance, |entity| {
        is_hitscan_blocker(engine, shooter, entity)
    })
}

pub(crate) fn is_hitscan_blocker(engine: &Engine, shooter: Actor, entity: Entity) -> bool {
    let Some(actor) = engine.actor(entity) else { return false; };
    if actor == shooter || actor.has::<ShooterPlayer>(engine) {
        return false;
    }

    // Only authored/physics solids should block bullets. Render-only helpers,
    // labels, trails, and editor gizmos must not affect gameplay line of sight.
    actor.has::<Collider>(engine)
        && (actor.has::<StaticBody>(engine)
            || actor.has::<KinematicBody>(engine)
            || actor.has::<RigidBody3D>(engine))
}
