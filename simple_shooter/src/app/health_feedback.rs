use super::*;

pub(crate) const LOW_HEALTH_THRESHOLD: f32 = 0.5;
pub(crate) const MAX_DAMAGE_VIGNETTE_STRENGTH: f32 = 0.85;
pub(crate) const DAMAGE_VIGNETTE_PASS_ID: &str = "simple_shooter/damage_vignette";

pub(crate) fn update_health_feedback(engine: &mut Engine, runtime: &ShooterRuntime) {
    let player = runtime.local_id
        .and_then(|id| find_player_actor(engine, id))
        .and_then(|actor| actor.get_component::<ShooterPlayer>(engine).cloned());
    let menu_active = engine.get_resource::<MainMenuState>().map(|menu| menu.active).unwrap_or(false);
    let Some(player) = player.filter(|player| player.alive && !menu_active) else {
        clear_health_feedback(engine);
        return;
    };

    let health = player.health.clamp(0, MAX_HEALTH);
    let health01 = health as f32 / MAX_HEALTH as f32;
    ensure_health_hud(engine);
    let color = health_hud_color(health01);
    let widgets = engine.actors_with::<HealthHudWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in widgets {
        if let Some(label) = actor.get_component_mut::<UILabel>(engine) {
            label.text = format!("HEALTH   {health} / {MAX_HEALTH}");
            label.color = color;
        }
        if let Some(panel) = actor.get_component_mut::<vetrace_ui::UIPanel>(engine) {
            panel.background = Vec3::new(0.018, 0.028, 0.04).lerp(Vec3::new(0.16, 0.018, 0.022), (1.0 - health01) * 0.65);
        }
    }

    let critical = ((LOW_HEALTH_THRESHOLD - health01) / LOW_HEALTH_THRESHOLD).clamp(0.0, 1.0);
    set_damage_vignette_strength(engine, critical * MAX_DAMAGE_VIGNETTE_STRENGTH);
}

pub(crate) fn ensure_health_hud(engine: &mut Engine) {
    if !engine.actors_with::<HealthHudWidget>().is_empty() { return; }
    engine.spawn_actor("Health HUD panel")
        .with(HealthHudWidget)
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.0, 1.0), offset_px: Vec2::new(126.0, -62.0), size_px: Vec2::new(220.0, 58.0), z_order: 330 })
        .with(vetrace_ui::UIPanel { size: Vec2::new(220.0, 58.0), background: Vec3::new(0.018, 0.028, 0.04), alpha: 0.9, anchor: Anchor::Center })
        .with(vetrace_ui::UIVisualStyle::rounded(12.0).with_border(1.0, Vec3::new(0.24, 0.78, 0.52), 0.8))
        .build();
    engine.spawn_actor("Health HUD label")
        .with(HealthHudWidget)
        .with(vetrace_ui::UIScreenSpace)
        .with(ScreenSpaceRect { anchor: Vec2::new(0.0, 1.0), offset_px: Vec2::new(126.0, -62.0), size_px: Vec2::new(200.0, 44.0), z_order: 331 })
        .with(UILabel { text: format!("HEALTH   {MAX_HEALTH} / {MAX_HEALTH}"), font_size: 20.0, color: health_hud_color(1.0), anchor: Anchor::Center, align: TextAlign::Center })
        .build();
}

pub(crate) fn health_hud_color(health01: f32) -> Vec3 {
    if health01 <= 0.25 { Vec3::new(1.0, 0.22, 0.24) }
    else if health01 <= 0.5 { Vec3::new(1.0, 0.68, 0.20) }
    else { Vec3::new(0.40, 1.0, 0.68) }
}

pub(crate) fn set_damage_vignette_strength(engine: &mut Engine, strength: f32) {
    if let Some(stack) = engine.get_resource_mut::<CustomPostProcessStack>() {
        if let Some(pass) = stack.passes.iter_mut().find(|pass| pass.pass_id == DAMAGE_VIGNETTE_PASS_ID) {
            if pass.params.len() < 4 { pass.params.resize(4, 0.0); }
            pass.params[0] = strength.clamp(0.0, 1.0);
            pass.enabled = pass.params[0] > 0.001;
        }
    }
}

pub(crate) fn clear_health_feedback(engine: &mut Engine) {
    let widgets = engine.actors_with::<HealthHudWidget>().into_iter().map(|(actor, _)| actor).collect::<Vec<_>>();
    for actor in widgets { actor.despawn(engine); }
    set_damage_vignette_strength(engine, 0.0);
}

#[cfg(test)]
mod health_feedback_tests {
    use super::*;

    #[test]
    fn health_colors_progress_from_safe_to_critical() {
        assert_ne!(health_hud_color(1.0), health_hud_color(0.5));
        assert_ne!(health_hud_color(0.5), health_hud_color(0.25));
    }
}
