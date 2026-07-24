use super::*;

pub(crate) fn damage_player(engine: &mut Engine, target: Actor, damage: i32, killer_id: u64, killer_name: &str) -> bool {
    let mut died = false;
    let mut dead_player_id = 0;
    if let Some(player) = target.get_component_mut::<ShooterPlayer>(engine) {
        if !player.alive { return false; }
        let damage_dealt = damage.max(0).min(player.health.max(0));
        let killer_life_damage = record_life_damage(&mut player.life_damage_by_attacker, killer_id, damage_dealt);
        player.health -= damage_dealt;
        if player.health <= 0 {
            player.health = 0;
            player.alive = false;
            player.respawn_timer = RESPAWN_DELAY;
            dead_player_id = player.id;
            player.deaths = player.deaths.saturating_add(1);
            player.last_killer_id = Some(killer_id);
            player.last_killer_name = killer_name.to_string();
            player.last_kill_damage = killer_life_damage;
            died = true;
            println!("{} died", player.name);
        }
    }
    if died {
        // Death is server-authoritative. Move the dead body to its spawn point
        // immediately, while keeping it invisible until the respawn timer ends.
        // This makes death/respawn obvious on the killed player's own client and
        // prevents remote clients from seeing the player reappear where they died.
        let spawn = spawn_position_for_active_map(engine, dead_player_id);
        teleport_player_body(engine, target, spawn, Vec3::ZERO);
        if let Some(stats) = engine.get_resource_mut::<ShooterStats>() {
            stats.deaths = stats.deaths.saturating_add(1);
        }
        set_player_visible(engine, target, false);
    }
    died
}

pub(crate) fn record_life_damage(sources: &mut Vec<(u64, i32)>, attacker_id: u64, damage: i32) -> i32 {
    if let Some((_, total)) = sources.iter_mut().find(|(id, _)| *id == attacker_id) {
        *total = (*total).saturating_add(damage);
        *total
    } else {
        sources.push((attacker_id, damage));
        damage
    }
}

#[cfg(test)]
mod damage_ledger_tests {
    use super::*;

    #[test]
    fn damage_totals_stay_separate_per_attacker() {
        let mut sources = Vec::new();
        assert_eq!(record_life_damage(&mut sources, 10, 25), 25);
        assert_eq!(record_life_damage(&mut sources, 20, 25), 25);
        assert_eq!(record_life_damage(&mut sources, 10, 25), 50);
    }
}

#[cfg(test)]
mod weapon_path_tests {
    use super::*;

    #[test]
    fn crosshair_mode_starts_at_muzzle_and_converges_on_target() {
        let muzzle = Vec3::new(0.4, -0.2, 0.0);
        let target = Vec3::new(0.0, 0.0, -10.0);
        let direction = physical_shot_direction(
            WeaponAimMode::CrosshairConverge,
            muzzle,
            Vec3::NEG_Z,
            Some(target),
            Vec3::NEG_Z,
        );
        assert!(direction.dot((target - muzzle).normalize()) > 0.9999);
    }

    #[test]
    fn barrel_mode_ignores_camera_target() {
        let barrel = Vec3::new(0.2, 0.1, -1.0).normalize();
        let direction = physical_shot_direction(
            WeaponAimMode::BarrelForward,
            Vec3::ZERO,
            barrel,
            Some(Vec3::X * 100.0),
            Vec3::NEG_Z,
        );
        assert!(direction.dot(barrel) > 0.9999);
    }
}
