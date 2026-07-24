use super::*;

pub(crate) fn spawn_player_outline_shell(engine: &mut Engine, owner: Actor) -> Actor {
    let style = ShooterOutlineStyle::default();
    engine
        .spawn_actor("Player Outline Shell")
        .with(ShooterOutlineShell)
        .with(ShooterOutlineOwner(owner))
        .with(PlayerVisualOwner { owner, kind: PlayerVisualKind::BodyOutline })
        .with(player_outline_shape(style, Vec3::ONE))
        .with(Material { base_color: style.color, alpha: 1.0, ..Material::default() })
        .with(player_outline_material(style))
        .with(Renderable { visible: false, ..Renderable::default() })
        .build()
}

pub(crate) fn player_outline_shape(style: ShooterOutlineStyle, owner_scale: Vec3) -> Shape {
    let grow_world = style.thickness.max(0.0) * 2.0;
    let safe_scale = owner_scale.abs().max(Vec3::splat(0.001));
    Shape {
        primitive: PrimitiveShape::Cube,
        size: Vec3::new(
            PLAYER_RADIUS * 2.0 + grow_world / safe_scale.x,
            PLAYER_VISUAL_HEIGHT + grow_world / safe_scale.y,
            PLAYER_RADIUS * 2.0 + grow_world / safe_scale.z,
        ),
    }
}

pub(crate) fn player_outline_material(style: ShooterOutlineStyle) -> CustomShaderMaterial {
    CustomShaderMaterial {
        shader_id: PLAYER_OUTLINE_SHADER_ID.to_string(),
        asset_path: Some(PLAYER_OUTLINE_SHADER_PATH.to_string()),
        wgsl_source: Some(PLAYER_OUTLINE_SHADER_SOURCE.to_string()),
        params: vec![style.color.x, style.color.y, style.color.z, 1.0],
        fallback_color_a: style.color,
        fallback_color_b: style.color,
        // Inverted-hull outline: draw only the expanded shell backfaces, but
        // keep the normal scene depth test so the shell cannot paint its far
        // plane over the real player/enemy body. Using `Always` here makes the
        // body look inside-out because the shell is drawn after the owner.
        cull_mode: CustomShaderCullMode::Front,
        depth_write: false,
        depth_compare: CustomShaderDepthCompare::LessEqual,
        render_bucket: CustomShaderRenderBucket::Overlay,
        ..CustomShaderMaterial::default()
    }
}

pub(crate) fn find_player_outline_shell(engine: &Engine, owner: Actor) -> Option<Actor> {
    engine.actors_with::<ShooterOutlineOwner>()
        .into_iter()
        .find_map(|(actor, marker)| (marker.0 == owner && actor.has::<ShooterOutlineShell>(engine)).then_some(actor))
}

pub(crate) fn despawn_orphan_outline_shells(engine: &mut Engine) {
    let shells = engine.actors_with::<ShooterOutlineOwner>()
        .into_iter()
        .map(|(actor, owner)| (actor, owner.0))
        .collect::<Vec<_>>();
    for (shell, owner) in shells {
        if !owner.is_alive(engine) || !owner.has::<ShooterPlayer>(engine) {
            shell.despawn(engine);
        }
    }
    let labels = engine.actors_with::<PlayerNameLabel>().into_iter()
        .filter_map(|(actor, _)| actor.get_component::<Parent>(engine).map(|parent| (actor, parent.0)))
        .collect::<Vec<_>>();
    for (label, owner_entity) in labels {
        let owner_is_player = engine.actor(owner_entity).map(|owner| owner.has::<ShooterPlayer>(engine)).unwrap_or(false);
        if !owner_is_player { label.despawn(engine); }
    }
}
