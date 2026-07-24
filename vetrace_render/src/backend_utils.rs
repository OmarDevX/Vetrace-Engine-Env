use super::*;

pub(super) fn rotated_direction(rotation: glam::Quat, direction: Vec3) -> Vec3 {
    let base = if direction.length_squared() > 1.0e-8 { direction.normalize() } else { Vec3::new(0.0, 0.0, -1.0) };
    let rotated = rotation * base;
    if rotated.length_squared() > 1.0e-8 { rotated.normalize() } else { base }
}


pub(super) fn egui_panel_from_debug_overlay(panel: &DebugTextOverlayPanel) -> EguiOverlayPanel {
    EguiOverlayPanel {
        enabled: panel.enabled,
        title: panel.title.clone(),
        subtitle: panel.subtitle.clone(),
        status: panel.status.clone(),
        lines: panel.lines.clone(),
        controls: panel.controls.clone(),
    }
}

pub(super) fn egui_input_snapshot_from_input(input: &InputState) -> EguiOverlayInputSnapshot {
    let (mx, my) = input.mouse_position();
    let (wx, wy) = input.mouse_wheel_delta();
    EguiOverlayInputSnapshot {
        mouse_position: [mx, my],
        mouse_wheel_delta: [wx, wy],
        left_pressed: input.was_mouse_button_pressed("Left"),
        left_released: input.was_mouse_button_released("Left"),
        right_pressed: input.was_mouse_button_pressed("Right"),
        right_released: input.was_mouse_button_released("Right"),
        middle_pressed: input.was_mouse_button_pressed("Middle"),
        middle_released: input.was_mouse_button_released("Middle"),
        shift: input.is_key_down("Shift"),
        ctrl: input.is_key_down("Control") || input.is_key_down("Ctrl"),
        alt: input.is_key_down("Alt"),
    }
}

pub(super) fn egui_keyboard_input_snapshot_from_input(input: &InputState) -> EguiOverlayKeyboardInputSnapshot {
    const EGUI_KEYS: &[&str] = &[
        "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowUp", "Escape", "Tab", "Backspace",
        "Enter", "Space", "Delete", "A", "B", "C", "D", "E", "F", "G", "H", "I",
        "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W",
        "X", "Y", "Z",
    ];
    let mut key_events = Vec::new();
    for key in EGUI_KEYS {
        if input.was_key_pressed(key) {
            key_events.push(EguiOverlayKeyEvent { key: (*key).to_string(), pressed: true });
        }
        if input.was_key_released(key) {
            key_events.push(EguiOverlayKeyEvent { key: (*key).to_string(), pressed: false });
        }
    }
    EguiOverlayKeyboardInputSnapshot {
        text_input: input.text_input().to_string(),
        key_events,
    }
}

pub(super) fn global_transform_for(engine: &Engine, entity: Entity) -> GlobalTransform {
    if let Some(global) = engine.raw_world().get::<GlobalTransform>(entity) {
        global.clone()
    } else if let Some(local) = engine.raw_world().get::<Transform>(entity) {
        GlobalTransform::from(local)
    } else {
        GlobalTransform::default()
    }
}


pub(super) fn global_transform_matrix(transform: &GlobalTransform) -> Mat4 {
    Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, transform.translation)
}

#[cfg(feature = "gltf_animation")]
pub(super) fn render_skin_for(engine: &Engine, entity: Entity, transform: &GlobalTransform) -> Option<RenderSkin> {
    let skin = engine.raw_world().get::<vetrace_animation::Skin>(entity)?;
    if skin.joints.is_empty() {
        return None;
    }

    let mesh_to_world = global_transform_matrix(transform);
    let world_to_mesh = mesh_to_world.inverse();
    if !world_to_mesh.to_cols_array().iter().all(|value| value.is_finite()) {
        return None;
    }

    let mut joint_matrices = Vec::with_capacity(skin.joints.len());
    for (index, joint) in skin.joints.iter().copied().enumerate() {
        let joint_global = global_transform_for(engine, joint);
        let joint_to_world = global_transform_matrix(&joint_global);
        let inverse_bind = skin.inverse_bind_matrices.get(index).copied().unwrap_or(Mat4::IDENTITY);
        joint_matrices.push(world_to_mesh * joint_to_world * inverse_bind);
    }
    Some(RenderSkin { joint_matrices })
}

#[cfg(not(feature = "gltf_animation"))]
pub(super) fn render_skin_for(_engine: &Engine, _entity: Entity, _transform: &GlobalTransform) -> Option<RenderSkin> {
    None
}

pub(super) fn material_for(engine: &Engine, entity: Entity, assets: Option<&RenderAssets>) -> Material {
    if let Some(material) = engine.raw_world().get::<Material>(entity) {
        return material.clone();
    }
    if let Some(renderable) = engine.raw_world().get::<Renderable>(entity) {
        if let (Some(assets), Some(handle)) = (assets, renderable.material) {
            if let Some(material) = assets.materials.get(&handle.0) {
                return material.clone();
            }
        }
    }
    if let Some(obj_mesh) = engine.raw_world().get::<ObjMesh>(entity) {
        if let (Some(assets), Some(handle)) = (assets, obj_mesh.material) {
            if let Some(material) = assets.materials.get(&handle.0) {
                return material.clone();
            }
        }
    }
    Material::default()
}

pub fn project_to_screen(point: Vec3, camera: &Camera, width: f32, height: f32) -> Option<Vec2> {
    let view = Mat4::look_at_rh(camera.position, camera.target, camera.up);
    let aspect = if height.abs() > f32::EPSILON { width / height } else { 1.0 };
    let projection = Mat4::perspective_rh(camera.fov_y_radians, aspect, camera.near, camera.far);
    let clip = projection * view * Vec4::new(point.x, point.y, point.z, 1.0);
    if clip.w.abs() <= f32::EPSILON { return None; }
    let ndc = clip.truncate() / clip.w;
    if ndc.z < -1.0 || ndc.z > 1.0 { return None; }
    Some(Vec2::new((ndc.x + 1.0) * 0.5 * width, (1.0 - ndc.y) * 0.5 * height))
}

pub fn primitive_radius(shape: Option<&Shape>, transform: &GlobalTransform) -> f32 {
    let base = match shape.map(|shape| shape.primitive).unwrap_or(PrimitiveShape::Cube) {
        PrimitiveShape::Cube => shape.map(|shape| shape.size.length() * 0.5).unwrap_or(0.866),
        PrimitiveShape::Sphere => shape.map(|shape| shape.size.max_element().abs() * 0.5).unwrap_or(0.5),
        PrimitiveShape::Capsule => shape.map(|shape| shape.size.max_element().abs() * 0.5).unwrap_or(0.5),
        PrimitiveShape::Plane | PrimitiveShape::Quad => shape.map(|shape| shape.size.truncate().length() * 0.5).unwrap_or(0.707),
    };
    base * transform.scale.max_element().abs().max(0.01)
}

pub fn material_color(material: &Material, light_count: usize) -> [u8; 4] {
    let light_boost = 0.65 + (light_count as f32).min(4.0) * 0.08;
    let emissive = material.emissive;
    let color = (material.base_color * light_boost + emissive).clamp(Vec3::ZERO, Vec3::ONE);
    [
        (color.x * 255.0) as u8,
        (color.y * 255.0) as u8,
        (color.z * 255.0) as u8,
        (material.alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

pub fn build_object_index(frame: &RenderFrame) -> HashMap<Entity, usize> {
    frame.objects.iter().enumerate().map(|(index, object)| (object.entity, index)).collect()
}
