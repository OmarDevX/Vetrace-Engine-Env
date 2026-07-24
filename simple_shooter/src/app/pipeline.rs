use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShooterFramePhase {
    InputAndIntent,
    AuthoritativeSimulation,
    Camera,
    Presentation,
    PostPhysicsSync,
    Cleanup,
}

pub(crate) const SHOOTER_FRAME_PHASES: &[ShooterFramePhase] = &[
    ShooterFramePhase::InputAndIntent,
    ShooterFramePhase::AuthoritativeSimulation,
    ShooterFramePhase::Camera,
    ShooterFramePhase::Presentation,
    ShooterFramePhase::PostPhysicsSync,
    ShooterFramePhase::Cleanup,
];

/// Presentation runs after authoritative simulation. Camera policy must run
/// before view/world weapon visibility, while shot results are consumed only
/// after simulation and networking have produced them.
pub(crate) fn run_presentation_stage(engine: &mut Engine, runtime: &ShooterRuntime, dt: f32) {
    if !engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) {
        engine.clear_events::<ShotResult>();
        return;
    }
    update_bullet_trails(engine, dt);
    update_muzzle_flashes(engine, dt);
    update_player_shader_params(engine, runtime.time);
    update_player_outline_styles(engine);
    update_camera(engine, runtime, dt);
    cleanup_orphan_player_visuals(engine);
    update_weapon_visuals(engine);
    present_pending_shots(engine);
    update_player_name_labels(engine);
    update_crosshair_entities(engine, runtime);
    update_health_feedback(engine, runtime);
}

pub(crate) fn run_post_physics_presentation_stage(engine: &mut Engine) {
    if !engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) { return; }
    update_player_outline_styles(engine);
    update_weapon_visuals(engine);
    update_player_name_labels(engine);
    vetrace_core::propagate_global_transforms(engine);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_precedes_presentation_and_cleanup_is_last() {
        let camera = SHOOTER_FRAME_PHASES.iter().position(|phase| *phase == ShooterFramePhase::Camera).unwrap();
        let presentation = SHOOTER_FRAME_PHASES.iter().position(|phase| *phase == ShooterFramePhase::Presentation).unwrap();
        assert!(camera < presentation);
        assert_eq!(SHOOTER_FRAME_PHASES.last(), Some(&ShooterFramePhase::Cleanup));
    }
}
