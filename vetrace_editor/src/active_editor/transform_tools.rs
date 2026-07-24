use super::*;

pub(crate) fn apply_keyboard_transform(engine: &mut Engine, input: &InputState, config: &EditorConfig, dt: f32) {
    let (selected, tool) = engine
        .get_resource::<EditorState>()
        .map(|state| (state.selected.clone(), state.active_tool))
        .unwrap_or_default();
    if selected.is_empty() { return; }

    let fast = if input.is_key_down("Shift") { 3.0 } else { 1.0 };
    let mut axis = Vec3::ZERO;
    if input.is_key_down("ArrowRight") || input.is_key_down("D") { axis.x += 1.0; }
    if input.is_key_down("ArrowLeft") || input.is_key_down("A") { axis.x -= 1.0; }
    if input.is_key_down("ArrowUp") || input.is_key_down("W") { axis.z -= 1.0; }
    if input.is_key_down("ArrowDown") || input.is_key_down("S") { axis.z += 1.0; }
    if input.is_key_down("E") { axis.y += 1.0; }
    if input.is_key_down("Q") { axis.y -= 1.0; }
    if axis == Vec3::ZERO { return; }

    for entity in selected {
        let mut updated_translation = None;
        {
            let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) else { continue; };
            match tool {
                EditorTool::Select | EditorTool::Translate | EditorTool::Omni => {
                    let delta = axis.normalize_or_zero() * config.translate_speed * fast * dt.max(0.0);
                    transform.translation += delta;
                }
                EditorTool::Rotate => {
                    let delta = axis * config.rotate_speed * fast * dt.max(0.0);
                    let rot = Quat::from_rotation_x(delta.z)
                        * Quat::from_rotation_y(-delta.x)
                        * Quat::from_rotation_z(delta.y);
                    transform.rotation = (rot * transform.rotation).normalize();
                }
                EditorTool::Scale => {
                    let amount = 1.0 + axis.length() * config.scale_speed * fast * dt.max(0.0);
                    let factor = if axis.x + axis.y + axis.z >= 0.0 { amount } else { 1.0 / amount.max(0.001) };
                    transform.scale = (transform.scale * factor).max(Vec3::splat(0.01));
                }
            }
            updated_translation = Some(transform.translation);
        }

        // No explicit Rapier write is needed here. The physics plugin treats
        // changed `Transform` on a physics-backed entity as an authoritative
        // external edit and pushes it into Rapier before the next simulation step.
        let _ = updated_translation;
    }
}

#[cfg(feature = "render_2d")]
pub(crate) fn apply_keyboard_transform_2d(
    engine: &mut Engine,
    input: &InputState,
    config: &EditorConfig,
    dt: f32,
) {
    let (selected, tool) = engine
        .get_resource::<EditorState>()
        .map(|state| (state.selected.clone(), state.active_tool))
        .unwrap_or_default();
    if selected.is_empty() { return; }

    let fast = if input.is_key_down("Shift") { 3.0 } else { 1.0 };
    let mut axis = Vec2::ZERO;
    if input.is_key_down("ArrowRight") || input.is_key_down("D") { axis.x += 1.0; }
    if input.is_key_down("ArrowLeft") || input.is_key_down("A") { axis.x -= 1.0; }
    if input.is_key_down("ArrowUp") || input.is_key_down("W") { axis.y += 1.0; }
    if input.is_key_down("ArrowDown") || input.is_key_down("S") { axis.y -= 1.0; }
    if axis == Vec2::ZERO { return; }

    for entity in selected {
        let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) else { continue; };
        match tool {
            EditorTool::Select | EditorTool::Translate | EditorTool::Omni => {
                let delta = axis.normalize_or_zero() * config.translate_speed * fast * dt.max(0.0);
                transform.translation.x += delta.x;
                transform.translation.y += delta.y;
            }
            EditorTool::Rotate => {
                let sign = if axis.x + axis.y >= 0.0 { 1.0 } else { -1.0 };
                let delta = sign * config.rotate_speed * fast * dt.max(0.0);
                transform.rotation = (Quat::from_rotation_z(delta) * transform.rotation).normalize();
            }
            EditorTool::Scale => {
                let sign = if axis.x + axis.y >= 0.0 { 1.0 } else { -1.0 };
                let amount = 1.0 + config.scale_speed * fast * dt.max(0.0);
                let factor = if sign >= 0.0 { amount } else { 1.0 / amount.max(0.001) };
                transform.scale.x = (transform.scale.x * factor).max(0.01);
                transform.scale.y = (transform.scale.y * factor).max(0.01);
                transform.scale.z = 1.0;
            }
        }
    }
}

#[cfg(feature = "render_2d")]
pub(crate) fn apply_pointer_transform_2d(engine: &mut Engine, input: &InputState) {
    if !input.is_mouse_button_down("Left") {
        return;
    }
    let tool = engine
        .get_resource::<EditorState>()
        .map(|state| state.active_tool)
        .unwrap_or_default();
    if matches!(tool, EditorTool::Select) {
        return;
    }
    let selected = engine
        .get_resource::<EditorState>()
        .map(|state| state.selected.clone())
        .unwrap_or_default();
    if selected.is_empty() {
        return;
    }
    let (dx, dy) = input.mouse_delta();
    if dx == 0.0 && dy == 0.0 {
        return;
    }

    let camera = engine.get_resource::<Camera2D>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let surface = Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32);
    let mouse = input.mouse_position();
    let current_screen = Vec2::new(mouse.0, mouse.1);
    let previous_screen = current_screen - Vec2::new(dx, dy);
    let current_world = camera.screen_to_world(current_screen, surface);
    let previous_world = camera.screen_to_world(previous_screen, surface);

    match tool {
        EditorTool::Select => {}
        EditorTool::Translate | EditorTool::Omni => {
            let world_delta = current_world - previous_world;
            for entity in selected {
                if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
                    transform.translation.x += world_delta.x;
                    transform.translation.y += world_delta.y;
                }
            }
        }
        EditorTool::Rotate => {
            for entity in selected {
                let pivot = global_transform_for(engine, entity).translation.truncate();
                let previous = previous_world - pivot;
                let current = current_world - pivot;
                if previous.length_squared() <= 1.0e-8 || current.length_squared() <= 1.0e-8 {
                    continue;
                }
                let raw_delta = current.y.atan2(current.x) - previous.y.atan2(previous.x);
                let delta = (raw_delta + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
                    transform.rotation = (Quat::from_rotation_z(delta) * transform.rotation).normalize();
                }
            }
        }
        EditorTool::Scale => {
            for entity in selected {
                let pivot = global_transform_for(engine, entity).translation.truncate();
                let previous_distance = (previous_world - pivot).length();
                let current_distance = (current_world - pivot).length();
                if previous_distance <= 1.0e-4 || !current_distance.is_finite() {
                    continue;
                }
                let factor = (current_distance / previous_distance).clamp(0.01, 100.0);
                if let Some(transform) = engine.raw_world_mut().get_mut::<Transform>(entity) {
                    let x = transform.scale.x * factor;
                    let y = transform.scale.y * factor;
                    transform.scale.x = if x < 0.0 { -x.abs().max(0.01) } else { x.max(0.01) };
                    transform.scale.y = if y < 0.0 { -y.abs().max(0.01) } else { y.max(0.01) };
                    transform.scale.z = 1.0;
                }
            }
        }
    }
}
