use super::*;

pub(crate) fn refresh_status(engine: &mut Engine) {
    let stats = engine.get_resource::<RenderStats>().cloned().unwrap_or_default();
    let selected_name = engine
        .get_resource::<EditorState>()
        .and_then(|state| state.selected_primary())
        .and_then(|entity| entity_label(engine, entity));
    let status = match selected_name {
        Some(name) => format!(
            "selected: {name} | visible objects: {} | lights: {} | frame: {}",
            stats.visible_objects, stats.directional_lights, stats.frames_rendered
        ),
        None => format!(
            "no selection | visible objects: {} | lights: {} | frame: {}",
            stats.visible_objects, stats.directional_lights, stats.frames_rendered
        ),
    };
    if let Some(state) = engine.get_resource_mut::<EditorState>() {
        state.status = status;
    }
}

pub(crate) fn refresh_egui_overlay(engine: &mut Engine, config: &EditorConfig) {
    let state = engine.get_resource::<EditorState>().cloned().unwrap_or_default();
    let stats = engine.get_resource::<RenderStats>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let entity_count = engine.raw_world().entities().count();
    let selected_label = state
        .selected_primary()
        .and_then(|entity| entity_label(engine, entity))
        .unwrap_or_else(|| "None".to_string());
    let hovered_label = state
        .hovered
        .and_then(|entity| entity_label(engine, entity))
        .unwrap_or_else(|| "None".to_string());
    let pick_text = state
        .last_pick_distance
        .map(|distance| format!("{distance:.2}m"))
        .unwrap_or_else(|| "n/a".to_string());

    engine.insert_resource(EguiOverlayPanel {
        enabled: true,
        title: "Vetrace Editor".to_string(),
        subtitle: "Active WGPU + egui overlay".to_string(),
        status: state.status.clone(),
        lines: vec![
            format!("Tool: {:?}", state.active_tool),
            format!("Gizmo space: {:?}", state.transform_space),
            format!("Multi pivot: {:?}", state.multi_pivot),
            format!("Selected: {selected_label}"),
            format!("Hovered: {hovered_label}"),
            format!("Pick distance: {pick_text}"),
            format!("Entities: {entity_count}"),
            format!("Visible objects: {}", stats.visible_objects),
            format!("Directional lights: {}", stats.directional_lights),
            format!("Frames rendered: {}", stats.frames_rendered),
            format!("Window: {}x{}", settings.width, settings.height),
            format!("Cursor unlocked: {}", config.unlock_cursor),
        ],
        controls: vec![
            "F10 - toggle shooter/editor mode".to_string(),
            "Left click - select object".to_string(),
            "Tab / Shift+Tab - cycle selection".to_string(),
            "G or 1 - translate tool".to_string(),
            "R or 2 - rotate tool".to_string(),
            "F or 3 - scale tool".to_string(),
            "T or 4 - omni gizmo".to_string(),
            "WASD / arrows - move selected on X/Z".to_string(),
            "Q/E - move selected down/up".to_string(),
            "Drag colored egui gizmo handles - transform selected".to_string(),
            "X - toggle local/global gizmo space".to_string(),
            "P - toggle multi-selection pivot".to_string(),
            "C - capture selected reflection probe".to_string(),
            "Shift - faster editing".to_string(),
            "Delete / Backspace - delete selected".to_string(),
            "Escape - deselect".to_string(),
        ],
    });
}
