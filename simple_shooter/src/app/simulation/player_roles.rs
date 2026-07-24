use super::*;

pub(crate) fn ensure_local_prediction_player(engine: &mut Engine, actor: Actor) {
    if !actor.has::<LocalPlayer>(engine) {
        actor.insert(engine, LocalPlayer).expect("player actor must be alive");
    }
    if !actor.has::<FirstPersonController>(engine) {
        actor.insert(engine, FirstPersonController::default()).expect("player actor must be alive");
    }
    if !actor.has::<FreeFlightController>(engine) {
        actor.insert(engine, FreeFlightController::default()).expect("player actor must be alive");
    }
    actor.remove::<RemotePlayer>(engine);
    actor.remove::<KinematicBody>(engine);

    if !actor.has::<RigidBody3D>(engine) {
        actor.insert(engine, RigidBody3D::default()).expect("player actor must be alive");
    }
    if !actor.has::<Velocity>(engine) {
        actor.insert(engine, Velocity::default()).expect("player actor must be alive");
    }
    if !actor.has::<AngularVelocity>(engine) {
        actor.insert(engine, AngularVelocity::default()).expect("player actor must be alive");
    }
    if !actor.has::<CharacterBody3D>(engine) {
        let mut character_body = CharacterBody3D::fps_capsule(PLAYER_RADIUS, PLAYER_HEIGHT);
        character_body.move_speed = MOVE_SPEED;
        character_body.jump_speed = JUMP_SPEED;
        actor.insert(engine, character_body).expect("player actor must be alive");
    }
    if !actor.has::<Collider>(engine) {
        actor
            .insert(engine, Collider {
                shape: ColliderShape::Capsule,
                half_extents: Vec3::new(PLAYER_RADIUS, PLAYER_HEIGHT * 0.5, PLAYER_RADIUS),
                ..Collider::default()
            })
            .expect("player actor must be alive");
    }
    sync_player_outline_style(engine, actor);
}

pub(crate) fn ensure_remote_snapshot_visual(engine: &mut Engine, actor: Actor) {
    actor.remove::<LocalPlayer>(engine);
    actor.remove::<FirstPersonController>(engine);
    actor.remove::<FreeFlightController>(engine);
    if !actor.has::<RemotePlayer>(engine) {
        actor.insert(engine, RemotePlayer).expect("player actor must be alive");
    }

    // Remote snapshot players on clients are not locally simulated characters,
    // but they still need a kinematic Rapier collider that follows the visible
    // interpolated transform. Otherwise the render body, physics body and hit
    // queries can disagree after player-to-player pushing/collision.
    actor.remove::<CharacterBody3D>(engine);
    actor.remove::<CharacterController3D>(engine);
    actor.remove::<CharacterControllerState>(engine);
    actor.remove::<RigidBody3D>(engine);
    actor.remove::<Velocity>(engine);
    actor.remove::<AngularVelocity>(engine);

    if !actor.has::<KinematicBody>(engine) {
        actor.insert(engine, KinematicBody::default()).expect("player actor must be alive");
    }
    if !actor.has::<Collider>(engine) {
        actor
            .insert(engine, Collider {
                shape: ColliderShape::Capsule,
                half_extents: Vec3::new(PLAYER_RADIUS, PLAYER_HEIGHT * 0.5, PLAYER_RADIUS),
                ..Collider::default()
            })
            .expect("player actor must be alive");
    }

    sync_player_outline_style(engine, actor);
}
