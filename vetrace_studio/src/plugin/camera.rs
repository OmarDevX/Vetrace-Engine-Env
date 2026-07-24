use super::*;

pub(super) fn authored_transform_signature(engine: &Engine) -> u64 {
    let mut entities = engine
        .raw_world()
        .entities()
        .filter(|entity| !engine.raw_world().has::<EditorOnly>(*entity))
        .collect::<Vec<_>>();
    entities.sort_unstable();
    let mut hash = 0xcbf29ce484222325_u64;
    for entity in entities {
        hash ^= entity.raw();
        hash = hash.wrapping_mul(0x100000001b3);
        let Some(transform) = engine.raw_world().get::<Transform>(entity) else { continue; };
        for value in transform
            .translation
            .to_array()
            .into_iter()
            .chain(transform.rotation.to_array())
            .chain(transform.scale.to_array())
        {
            hash ^= value.to_bits() as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

pub(super) fn update_studio_camera(engine: &mut Engine, pointer_captured: bool, dt: f32) {
    #[cfg(feature = "render_2d")]
    if engine
        .get_resource::<EditorState>()
        .map(|state| state.viewport_mode == vetrace_editor::EditorViewportMode::TwoD)
        .unwrap_or(false)
    {
        sync_studio_camera_2d_viewport(engine);
        if !pointer_captured {
            update_studio_camera_2d(engine);
        }
        return;
    }

    if pointer_captured { return; }
    update_studio_camera_3d(engine, dt);
}

fn update_studio_camera_3d(engine: &mut Engine, dt: f32) {
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    if !input.is_mouse_button_down("Right") { return; }
    let dt = if dt > 0.0 { dt.min(0.1) } else { 1.0 / 60.0 };
    let (dx, dy) = input.mouse_delta();
    let mut next = None;
    if let Some(state) = engine.get_resource_mut::<StudioCameraState>() {
        state.yaw += dx * 0.0022;
        state.pitch = (state.pitch - dy * 0.0022).clamp(-1.45, 1.45);
        if input.mouse_wheel_delta().1 != 0.0 {
            state.speed = (state.speed
                * if input.mouse_wheel_delta().1 > 0.0 { 1.15 } else { 1.0 / 1.15 })
                .clamp(0.5, 100.0);
        }
        let forward = Vec3::new(
            state.yaw.cos() * state.pitch.cos(),
            state.pitch.sin(),
            state.yaw.sin() * state.pitch.cos(),
        ).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let mut movement = Vec3::ZERO;
        if input.is_key_down("W") { movement += forward; }
        if input.is_key_down("S") { movement -= forward; }
        if input.is_key_down("D") { movement += right; }
        if input.is_key_down("A") { movement -= right; }
        if input.is_key_down("E") { movement += Vec3::Y; }
        if input.is_key_down("Q") { movement -= Vec3::Y; }
        let boost = if input.is_key_down("Shift") { 3.5 } else { 1.0 };
        next = Some((forward, movement.normalize_or_zero() * state.speed * boost * dt));
    }
    if let Some((forward, movement)) = next {
        if let Some(camera) = engine.get_resource_mut::<Camera>() {
            camera.position += movement;
            camera.target = camera.position + forward;
            camera.up = Vec3::Y;
        }
    }
}


#[cfg(feature = "render_2d")]
fn sync_studio_camera_2d_viewport(engine: &mut Engine) {
    let bounds = engine
        .get_resource::<EditorViewportBounds>()
        .copied()
        .unwrap_or_default();
    let Some(camera) = engine.get_resource_mut::<Camera2D>() else { return; };
    match bounds.0.filter(|rect| !rect.is_empty()) {
        Some(rect) => camera.set_viewport_px(
            Vec2::new(rect.min_x, rect.min_y),
            Vec2::new(rect.width(), rect.height()),
        ),
        None => camera.clear_viewport(),
    }
}

#[cfg(feature = "render_2d")]
fn update_studio_camera_2d(engine: &mut Engine) {
    let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let viewport = Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32);
    let mouse = input.mouse_position();
    let mouse = Vec2::new(mouse.0, mouse.1);
    let (dx, dy) = input.mouse_delta();
    let wheel = input.mouse_wheel_delta().1;

    let Some(camera) = engine.get_resource_mut::<Camera2D>() else { return; };
    if input.is_mouse_button_down("Right") {
        let local_delta = Vec2::new(dx, -dy) / camera.pixels_per_world_unit();
        camera.position -= glam::Mat2::from_angle(camera.rotation) * local_delta;
    }

    if wheel.abs() > f32::EPSILON {
        let world_under_cursor = camera.screen_to_world(mouse, viewport);
        let zoom_factor = (wheel * 0.12).exp().clamp(0.25, 4.0);
        camera.zoom = (camera.zoom * zoom_factor).clamp(0.01, 256.0);
        let world_after_zoom = camera.screen_to_world(mouse, viewport);
        camera.position += world_under_cursor - world_after_zoom;
    }
}
