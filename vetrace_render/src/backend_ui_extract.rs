use super::*;

pub(super) fn push_world_ui_elements(
    engine: &Engine,
    entity: Entity,
    world_position: Vec3,
    placement: RenderWorldUiPlacement,
    out: &mut Vec<RenderWorldUiElement>,
) {
    for (slot, kind) in ui_kinds_for_entity(engine, entity, placement.size_px) {
        out.push(RenderWorldUiElement {
            entity,
            slot,
            world_position,
            placement: placement.clone(),
            kind,
        });
    }
}

#[cfg(feature = "egui_render")]
pub(super) fn push_screen_ui_elements(engine: &Engine, entity: Entity, rect: ScreenSpaceRect, out: &mut Vec<RenderScreenUiElement>) {
    let style = engine.raw_world().get::<vetrace_ui::UIVisualStyle>(entity)
        .copied()
        .unwrap_or_default()
        .into();
    for (slot, kind) in ui_kinds_for_entity(engine, entity, rect.size_px) {
        out.push(RenderScreenUiElement {
            entity,
            slot,
            rect: rect.clone(),
            kind,
            style,
        });
    }
}

#[cfg(feature = "egui_render")]
pub(super) fn ui_kinds_for_entity(engine: &Engine, entity: Entity, size_hint_px: Vec2) -> Vec<(u8, RenderScreenUiKind)> {
    let mut kinds = Vec::new();

    if let Some(panel) = engine.raw_world().get::<vetrace_ui::UIPanel>(entity) {
        kinds.push((0, RenderScreenUiKind::Panel {
            size_px: nonzero_size(panel.size, size_hint_px, Vec2::new(160.0, 80.0)),
            background: panel.background,
            alpha: panel.alpha,
        }));
    }

    if let Some(label) = engine.raw_world().get::<vetrace_ui::UILabel>(entity) {
        if !label.text.trim().is_empty() {
            kinds.push((1, RenderScreenUiKind::Label {
                text: label.text.clone(),
                font_size: label.font_size,
                color: label.color,
                align: label.align,
            }));
        }
    }

    if let Some(button) = engine.raw_world().get::<vetrace_ui::UIButton>(entity) {
        let default_bg = if !button.enabled {
            Vec3::new(0.23, 0.24, 0.27)
        } else if button.pressed {
            Vec3::new(0.08, 0.20, 0.58)
        } else if button.hovered {
            Vec3::new(0.16, 0.38, 0.92)
        } else {
            Vec3::new(0.12, 0.30, 0.78)
        };
        let (background, alpha) = ui_material_background(engine, entity, default_bg, 0.95);
        kinds.push((2, RenderScreenUiKind::Button {
            text: button.text.clone(),
            size_px: nonzero_size(button.size, size_hint_px, Vec2::new(120.0, 32.0)),
            background,
            alpha,
            enabled: button.enabled,
            hovered: button.hovered,
            pressed: button.pressed,
        }));
    }

    if let Some(editor) = engine.raw_world().get::<vetrace_ui::UITextEditor>(entity) {
        let default_bg = if editor.focused { Vec3::new(0.13, 0.15, 0.20) } else { Vec3::new(0.10, 0.11, 0.13) };
        let (background, alpha) = ui_material_background(engine, entity, default_bg, 0.92);
        kinds.push((3, RenderScreenUiKind::TextEditor {
            text: editor.text.clone(),
            placeholder: editor.placeholder.clone(),
            size_px: nonzero_size(size_hint_px, Vec2::ZERO, Vec2::new(180.0, 32.0)),
            background,
            alpha,
            focused: editor.focused,
            multiline: editor.multiline,
        }));
    }

    if let Some(list) = engine.raw_world().get::<vetrace_ui::UIList>(entity) {
        kinds.push((4, RenderScreenUiKind::List {
            items: list.items.clone(),
            selected: list.selected,
            size_px: nonzero_size(size_hint_px, Vec2::ZERO, Vec2::new(180.0, 120.0)),
        }));
    }

    if let Some(rect) = engine.raw_world().get::<vetrace_ui::ColorRect>(entity) {
        kinds.push((5, RenderScreenUiKind::ColorRect {
            size_px: nonzero_size(rect.size, size_hint_px, Vec2::new(100.0, 100.0)),
            color: rect.color,
            alpha: rect.alpha,
        }));
    }

    kinds
}

#[cfg(feature = "egui_render")]
pub(super) fn ui_material_background(engine: &Engine, entity: Entity, fallback: Vec3, fallback_alpha: f32) -> (Vec3, f32) {
    engine.raw_world().get::<Material>(entity)
        .map(|material| (material.base_color, material.alpha))
        .unwrap_or((fallback, fallback_alpha))
}

#[cfg(feature = "egui_render")]
pub(super) fn nonzero_size(primary: Vec2, secondary: Vec2, fallback: Vec2) -> Vec2 {
    if primary.x > 0.0 && primary.y > 0.0 {
        primary
    } else if secondary.x > 0.0 && secondary.y > 0.0 {
        secondary
    } else {
        fallback
    }
}
