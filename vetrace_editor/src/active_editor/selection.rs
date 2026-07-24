use super::*;

pub(crate) fn set_tool(engine: &mut Engine, tool: EditorTool) {
    if let Some(state) = engine.get_resource_mut::<EditorState>() {
        state.active_tool = tool;
    }
}

pub(crate) fn set_selected(engine: &mut Engine, entity: Option<Entity>, config: &EditorConfig) {
    restore_editor_outlines(engine);
    if let Some(state) = engine.get_resource_mut::<EditorState>() {
        state.set_selected(entity);
    }
    apply_editor_outlines(engine, config);
}

pub(crate) fn cycle_selection(engine: &mut Engine, config: &EditorConfig, direction: i32) {
    let candidates = selectable_entities(engine);
    if candidates.is_empty() {
        set_selected(engine, None, config);
        return;
    }
    let current = engine
        .get_resource::<EditorState>()
        .and_then(|state| state.selected_primary());
    let current_index = current
        .and_then(|entity| candidates.iter().position(|candidate| *candidate == entity))
        .unwrap_or_else(|| if direction < 0 { 0 } else { candidates.len().saturating_sub(1) });
    let len = candidates.len() as i32;
    let next = ((current_index as i32 + direction).rem_euclid(len)) as usize;
    set_selected(engine, Some(candidates[next]), config);
}

pub(crate) fn selectable_entities(engine: &Engine) -> Vec<Entity> {
    engine.raw_world().entities()
        .filter(|entity| {
            !engine.raw_world().has::<EditorOnly>(*entity)
                && (engine.raw_world().has::<Renderable>(*entity)
                    || engine.raw_world().has::<Shape>(*entity)
                    || {
                        #[cfg(feature = "render_2d")]
                        {
                            engine.raw_world().has::<Sprite2D>(*entity)
                                || {
                                    #[cfg(feature = "physics_2d")]
                                    { engine.raw_world().has::<Collider2D>(*entity) }
                                    #[cfg(not(feature = "physics_2d"))]
                                    { false }
                                }
                        }
                        #[cfg(not(feature = "render_2d"))]
                        { false }
                    })
        })
        .collect()
}

pub(crate) fn apply_editor_outlines(engine: &mut Engine, config: &EditorConfig) {
    if !config.draw_selection_outline { return; }
    let selected = engine
        .get_resource::<EditorState>()
        .map(|state| state.selected.clone())
        .unwrap_or_default();
    for entity in selected {
        if !engine.raw_world().is_alive(entity) { continue; }
        if !engine.raw_world().has::<Renderable>(entity) && !engine.raw_world().has::<Shape>(entity) { continue; }
        let old_outline = engine.raw_world().get::<Outline>(entity).cloned();
        if let Some(backups) = engine.get_resource_mut::<EditorOutlineBackups>() {
            backups.previous.entry(entity).or_insert(old_outline);
        }
        engine.raw_world_mut().insert(entity, Outline {
            enabled: true,
            color: config.selection_outline_color,
            thickness: config.selection_outline_thickness,
        });
    }
}

pub(crate) fn restore_editor_outlines(engine: &mut Engine) {
    let backups = engine
        .remove_resource::<EditorOutlineBackups>()
        .unwrap_or_default();
    for (entity, previous) in backups.previous {
        if !engine.raw_world().is_alive(entity) { continue; }
        match previous {
            Some(outline) => engine.raw_world_mut().insert(entity, outline),
            None => {
                let _ = engine.raw_world_mut().remove::<Outline>(entity);
            }
        }
    }
    engine.insert_resource(EditorOutlineBackups::default());
}

pub(crate) fn delete_selected(engine: &mut Engine) {
    restore_editor_outlines(engine);
    let selected = engine
        .get_resource::<EditorState>()
        .map(|state| state.selected.clone())
        .unwrap_or_default();
    for entity in selected {
        // The physics plugin now observes despawn/component removal and cleans
        // its Rapier body/collider maps automatically on the next physics tick.
        engine.raw_world_mut().despawn(entity);
    }
    if let Some(state) = engine.get_resource_mut::<EditorState>() {
        state.clear_selection();
    }
}
