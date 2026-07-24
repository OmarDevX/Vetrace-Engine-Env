use super::*;

const EDGE_COUNT: usize = 4;
const OVERLAY_Z: i32 = 1_000_000;

pub(crate) struct Editor2DSelectionOverlay {
    edges: [Entity; EDGE_COUNT],
}

pub(crate) fn install_2d_selection_overlay(engine: &mut Engine) {
    if engine.contains_resource::<Editor2DSelectionOverlay>() {
        return;
    }
    let edges = std::array::from_fn(|index| {
        let actor = engine
            .spawn_actor(format!("Editor 2D Selection Edge {index}"))
            .with(EditorOnly)
            .with(ScreenSpaceRect {
                anchor: Vec2::ZERO,
                offset_px: Vec2::ZERO,
                size_px: Vec2::ZERO,
                z_order: OVERLAY_Z,
            })
            .with(Material {
                alpha: 0.0,
                ..Material::default()
            })
            .source("vetrace_editor")
            .build();
        let _ = actor.add_tag(engine, "map_builder_helper");
        actor.entity()
    });
    engine.insert_resource(Editor2DSelectionOverlay { edges });
}

pub(crate) fn hide_2d_selection_overlay(engine: &mut Engine) {
    let Some(edges) = engine
        .get_resource::<Editor2DSelectionOverlay>()
        .map(|overlay| overlay.edges)
    else {
        return;
    };
    for edge in edges {
        if let Some(rect) = engine.raw_world_mut().get_mut::<ScreenSpaceRect>(edge) {
            rect.size_px = Vec2::ZERO;
        }
        if let Some(material) = engine.raw_world_mut().get_mut::<Material>(edge) {
            material.alpha = 0.0;
        }
    }
}

pub(crate) fn refresh_2d_selection_overlay(engine: &mut Engine, config: &EditorConfig) {
    let in_2d = engine
        .get_resource::<EditorState>()
        .map(|state| state.viewport_mode == EditorViewportMode::TwoD)
        .unwrap_or(false);
    if !in_2d || !config.draw_selection_outline {
        hide_2d_selection_overlay(engine);
        return;
    }

    let Some(entity) = engine
        .get_resource::<EditorState>()
        .and_then(EditorState::selected_primary)
    else {
        hide_2d_selection_overlay(engine);
        return;
    };
    let sprite = engine.raw_world().get::<Sprite2D>(entity).cloned();
    let sprite_visible = sprite.as_ref().is_some_and(|_| {
        engine
            .raw_world()
            .get::<CanvasItem2D>(entity)
            .cloned()
            .unwrap_or_default()
            .visible
    });
    #[cfg(feature = "physics_2d")]
    let collider = engine
        .raw_world()
        .get::<Collider2D>(entity)
        .filter(|collider| collider.enabled)
        .cloned();
    #[cfg(not(feature = "physics_2d"))]
    let collider: Option<()> = None;

    let local_corners = if sprite_visible {
        let sprite = sprite.as_ref().expect("visible sprite exists");
        let local_min = -sprite.size * sprite.pivot;
        let local_max = local_min + sprite.size;
        [
            Vec2::new(local_min.x, local_min.y),
            Vec2::new(local_max.x, local_min.y),
            Vec2::new(local_max.x, local_max.y),
            Vec2::new(local_min.x, local_max.y),
        ]
    } else {
        #[cfg(feature = "physics_2d")]
        {
            let Some(collider) = collider else {
                hide_2d_selection_overlay(engine);
                return;
            };
            collider_local_corners(&collider)
        }
        #[cfg(not(feature = "physics_2d"))]
        {
            let _ = collider;
            hide_2d_selection_overlay(engine);
            return;
        }
    };

    let transform = global_transform_for(engine, entity);
    let camera = engine.get_resource::<Camera2D>().cloned().unwrap_or_default();
    let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
    let viewport = Vec2::new(settings.width.max(1) as f32, settings.height.max(1) as f32);
    let mut screen_min = Vec2::splat(f32::INFINITY);
    let mut screen_max = Vec2::splat(f32::NEG_INFINITY);
    for corner in local_corners {
        let world = transform.translation
            + transform.rotation * Vec3::new(
                corner.x * transform.scale.x,
                corner.y * transform.scale.y,
                0.0,
            );
        let screen = camera.world_to_screen(world.truncate(), viewport);
        screen_min = screen_min.min(screen);
        screen_max = screen_max.max(screen);
    }
    if !screen_min.is_finite() || !screen_max.is_finite() {
        hide_2d_selection_overlay(engine);
        return;
    }

    let thickness = (config.selection_outline_thickness.abs() * 32.0).clamp(1.5, 4.0);
    let width = (screen_max.x - screen_min.x).max(thickness);
    let height = (screen_max.y - screen_min.y).max(thickness);
    let center = (screen_min + screen_max) * 0.5;
    let rects = [
        (Vec2::new(center.x, screen_min.y + thickness * 0.5), Vec2::new(width, thickness)),
        (Vec2::new(center.x, screen_max.y - thickness * 0.5), Vec2::new(width, thickness)),
        (Vec2::new(screen_min.x + thickness * 0.5, center.y), Vec2::new(thickness, height)),
        (Vec2::new(screen_max.x - thickness * 0.5, center.y), Vec2::new(thickness, height)),
    ];
    let Some(edges) = engine
        .get_resource::<Editor2DSelectionOverlay>()
        .map(|overlay| overlay.edges)
    else {
        return;
    };
    for (edge, (offset, size)) in edges.into_iter().zip(rects) {
        if let Some(rect) = engine.raw_world_mut().get_mut::<ScreenSpaceRect>(edge) {
            rect.anchor = Vec2::ZERO;
            rect.offset_px = offset;
            rect.size_px = size;
            rect.z_order = OVERLAY_Z;
        }
        if let Some(material) = engine.raw_world_mut().get_mut::<Material>(edge) {
            material.base_color = config.selection_outline_color;
            material.emissive = Vec3::ZERO;
            material.alpha = 1.0;
        }
    }
}


#[cfg(feature = "physics_2d")]
fn collider_local_corners(collider: &Collider2D) -> [Vec2; 4] {
    let half = match collider.shape {
        ColliderShape2D::Circle => Vec2::splat(collider.radius.abs()),
        ColliderShape2D::Box => collider.half_extents.abs(),
    };
    let corners = [
        Vec2::new(-half.x, -half.y),
        Vec2::new(half.x, -half.y),
        Vec2::new(half.x, half.y),
        Vec2::new(-half.x, half.y),
    ];
    let (sin, cos) = collider.rotation.sin_cos();
    corners.map(|corner| {
        Vec2::new(corner.x * cos - corner.y * sin, corner.x * sin + corner.y * cos)
            + collider.offset
    })
}
