use super::*;

pub(crate) fn key_pressed_any(input: &InputState, names: &[&str]) -> bool {
    names.iter().any(|name| input.was_key_pressed(name))
}

pub(crate) fn key_down_any(input: &InputState, names: &[&str]) -> bool {
    names.iter().any(|name| input.is_key_down(name))
}

pub(crate) fn handle_free_flight_toggle_and_speed(engine: &mut Engine, runtime: &ShooterRuntime) {
    if engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false) { return; }
    if runtime.editor_enabled {
        return;
    }
    let Some(local_id) = runtime.local_id else { return; };
    let Some(actor) = find_player_actor(engine, local_id) else { return; };
        if !actor.has::<FreeFlightController>(engine) {
        actor.insert(engine, FreeFlightController::default()).expect("local player actor must be alive");
    }

    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let toggle_requested = input.was_mouse_button_pressed("Right") || key_pressed_any(&input, &["F", "SDLK_102"]);
    if toggle_requested {
        toggle_free_flight(engine, actor);
    }

    let enabled = actor.get_component::<FreeFlightController>(engine).map(|controller| controller.enabled).unwrap_or(false);
    if !enabled {
        return;
    }

    let shift = input.is_key_down("Shift");
    let mut speed_steps = 0.0_f32;
    let key_step = if shift { 4.0 } else { 1.0 };
    if key_pressed_any(&input, &["Digit5", "ArrowUp", "SDLK_53", "SDLK_1073741906"]) {
        speed_steps += key_step;
    }
    if key_pressed_any(&input, &["Digit4", "ArrowDown", "SDLK_52", "SDLK_1073741905"]) {
        speed_steps -= key_step;
    }
    let (_, wheel_y) = input.mouse_wheel_delta();
    if wheel_y.abs() > 0.0 {
        speed_steps += wheel_y * if shift { 2.0 } else { 0.5 };
    }

    if speed_steps.abs() > 0.0 {
        if let Some(controller) = actor.get_component_mut::<FreeFlightController>(engine) {
            controller.speed = (controller.speed * FREE_FLIGHT_SPEED_STEP.powf(speed_steps))
                .clamp(FREE_FLIGHT_MIN_SPEED, FREE_FLIGHT_MAX_SPEED);
            println!("free flight speed: {:.3}", controller.speed);
        }
    }
}

pub(crate) fn toggle_free_flight(engine: &mut Engine, actor: Actor) {
    let was_enabled = actor.get_component::<FreeFlightController>(engine).map(|controller| controller.enabled).unwrap_or(false);
    if was_enabled {
        let pose = actor.get_component::<FreeFlightController>(engine).map(|controller| (controller.yaw, controller.pitch));
        if let Some(controller) = actor.get_component_mut::<FreeFlightController>(engine) {
            controller.enabled = false;
            controller.velocity = Vec3::ZERO;
        }
        if let Some((yaw, pitch)) = pose {
            if let Some(fps) = actor.get_component_mut::<FirstPersonController>(engine) {
                fps.yaw = yaw;
                fps.pitch = pitch;
            }
            if let Some(player) = actor.get_component_mut::<ShooterPlayer>(engine) {
                player.yaw = yaw;
                player.pitch = pitch;
            }
        }
        println!("free flight: off");
        return;
    }

    let player_pose = actor.get_component::<ShooterPlayer>(engine).map(|player| (player.yaw, player.pitch));
    let controller_pose = actor.get_component::<FirstPersonController>(engine).map(|controller| (controller.yaw, controller.pitch));
    let transform_position = actor.get_component::<Transform>(engine).map(|transform| player_eye_position(transform.translation));
    let camera_position = engine.get_resource::<Camera>().map(|camera| camera.position);
    let (yaw, pitch) = controller_pose.or(player_pose).unwrap_or((0.0, 0.0));
    let position = camera_position.or(transform_position).unwrap_or(Vec3::new(0.0, FPS_EYE_HEIGHT, 6.0));

    if let Some(controller) = actor.get_component_mut::<FreeFlightController>(engine) {
        controller.enabled = true;
        controller.position = position;
        controller.yaw = yaw;
        controller.pitch = pitch;
        controller.velocity = Vec3::ZERO;
        println!("free flight: on | speed: {:.3}", controller.speed);
    }
}

pub(crate) fn update_free_flight_controller(engine: &mut Engine, runtime: &ShooterRuntime, dt: f32) {
    if engine.get_resource::<PauseMenuState>().map(|state| state.active).unwrap_or(false) { return; }
    if runtime.editor_enabled {
        return;
    }
    let Some(local_id) = runtime.local_id else { return; };
    let Some(actor) = find_player_actor(engine, local_id) else { return; };
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let Some(controller) = actor.get_component_mut::<FreeFlightController>(engine) else { return; };
    if !controller.enabled {
        return;
    }

    let (mouse_dx, mouse_dy) = input.mouse_delta();
    controller.yaw += mouse_dx * controller.sensitivity;
    controller.pitch = (controller.pitch - mouse_dy * controller.sensitivity).clamp(-1.50, 1.50);

    if input.is_key_down("Space") {
        controller.velocity = Vec3::ZERO;
    }

    let forward = forward_from_angles(controller.yaw, controller.pitch);
    let right = right_from_yaw(controller.yaw);
    let mut intent = Vec3::ZERO;
    if input.is_key_down("W") { intent += forward; }
    if input.is_key_down("S") { intent -= forward; }
    if input.is_key_down("D") { intent += right; }
    if input.is_key_down("A") { intent -= right; }
    if key_down_any(&input, &["E", "SDLK_101"]) { intent += Vec3::Y; }
    if key_down_any(&input, &["Q", "SDLK_113"]) { intent -= Vec3::Y; }

    let boost = if input.is_key_down("Shift") { FREE_FLIGHT_SHIFT_MULTIPLIER } else { 1.0 };
    let slow = if key_down_any(&input, &["Control", "Ctrl"]) { FREE_FLIGHT_CONTROL_MULTIPLIER } else { 1.0 };
    let max_speed = (controller.speed * boost * slow).clamp(FREE_FLIGHT_MIN_SPEED, FREE_FLIGHT_MAX_SPEED);
    let dt = dt.clamp(0.0, 0.1);

    if intent.length_squared() > 1.0e-6 {
        let accel = intent.normalize() * controller.acceleration * boost * slow;
        controller.velocity += accel * dt;
        controller.velocity = controller.velocity.clamp_length_max(max_speed);
        controller.velocity *= controller.friction.powf(dt).clamp(0.0, 1.0);
    } else {
        let speed = controller.velocity.length();
        if speed > 0.0 {
            let new_speed = (speed - controller.deceleration * dt).max(0.0);
            controller.velocity = if new_speed <= 0.0001 { Vec3::ZERO } else { controller.velocity / speed * new_speed };
        }
    }

    controller.position += controller.velocity * dt;
}

pub(crate) fn local_player_is_free_flying(engine: &Engine, player_id: u64) -> bool {
    find_player_actor(engine, player_id)
        .and_then(|actor| actor.get_component::<FreeFlightController>(engine))
        .map(|controller| controller.enabled)
        .unwrap_or(false)
}

pub(crate) fn spawn_crosshair(engine: &mut Engine) {
    spawn_crosshair_part(engine, "Crosshair Horizontal", true);
    spawn_crosshair_part(engine, "Crosshair Vertical", false);
}

pub(crate) fn spawn_crosshair_part(engine: &mut Engine, name: &str, horizontal: bool) -> Actor {
    engine
        .spawn_actor(name)
        .with(CrosshairPart { horizontal })
        .with(ScreenSpaceRect {
            anchor: Vec2::new(0.5, 0.5),
            offset_px: Vec2::ZERO,
            size_px: if horizontal { Vec2::new(24.0, 2.0) } else { Vec2::new(2.0, 24.0) },
            z_order: 100,
        })
        .with(Material {
            base_color: Vec3::new(0.88, 0.96, 1.0),
            emissive: Vec3::new(0.22, 0.36, 0.55),
            roughness: 0.2,
            ..Material::default()
        })
        .with(Renderable { visible: true, ..Renderable::default() })
        .build()
}
