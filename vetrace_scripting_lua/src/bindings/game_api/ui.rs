use super::*;

pub(super) fn install(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let ui = lua.create_table()?;
    ui.set("contains", lua.create_function(|_, (entity, x, y): (AnyUserData, Option<f32>, Option<f32>)| {
        let proxy = entity.borrow::<EntityProxy>()?;
        let Some(entity) = resolve_entity_now(proxy.target)? else { return Ok(false); };
        with_context(|engine, _, _, _, _, _| {
            let Some(rect) = engine.raw_world().get::<ScreenSpaceRect>(entity) else { return Ok(false); };
            let viewport = engine.get_resource::<RenderSettings>()
                .map(|settings| glam::Vec2::new(settings.width as f32, settings.height as f32))
                .unwrap_or(glam::Vec2::new(1280.0, 720.0));
            let point = match (x, y) {
                (Some(x), Some(y)) => glam::Vec2::new(x, y),
                _ => engine.get_resource::<InputState>()
                    .map(|input| { let (x, y) = input.mouse_position(); glam::Vec2::new(x, y) })
                    .unwrap_or(glam::Vec2::ZERO),
            };
            Ok(vetrace_ui::screen_rect_contains(viewport, rect.anchor, rect.offset_px, rect.size_px, point))
        })
    })?)?;
    ui.set("button", lua.create_function(|lua, entity: AnyUserData| {
        let proxy = entity.borrow::<EntityProxy>()?;
        let Some(entity) = resolve_entity_now(proxy.target)? else {
            let result = lua.create_table()?;
            result.set("hovered", false)?;
            result.set("pressed", false)?;
            result.set("clicked", false)?;
            return Ok(result);
        };
        with_context(|engine, _, _, _, _, _| {
            let rect = engine.raw_world().get::<ScreenSpaceRect>(entity).cloned()
                .ok_or_else(|| mlua::Error::external("UI.button entity has no vetrace.render.screen_space_rect"))?;
            let viewport = engine.get_resource::<RenderSettings>()
                .map(|settings| glam::Vec2::new(settings.width as f32, settings.height as f32))
                .unwrap_or(glam::Vec2::new(1280.0, 720.0));
            let (point, pointer_down, pointer_released) = engine.get_resource::<InputState>()
                .map(|input| {
                    let (x, y) = input.mouse_position();
                    (glam::Vec2::new(x, y), input.is_mouse_button_down("Left"), input.was_mouse_button_released("Left"))
                })
                .unwrap_or((glam::Vec2::ZERO, false, false));
            let enabled = engine.raw_world().get::<UIButton>(entity).map(|button| button.enabled).unwrap_or(false);
            let mut interaction = vetrace_ui::pointer_interaction(
                viewport, rect.anchor, rect.offset_px, rect.size_px, point, pointer_down, pointer_released,
            );
            if !enabled {
                interaction = vetrace_ui::UIInteraction::default();
            }
            let button = engine.raw_world_mut().get_mut::<UIButton>(entity)
                .ok_or_else(|| mlua::Error::external("UI.button entity has no vetrace.ui.button"))?;
            button.hovered = interaction.hovered;
            button.pressed = interaction.pressed;
            let result = lua.create_table()?;
            result.set("hovered", interaction.hovered)?;
            result.set("pressed", interaction.pressed)?;
            result.set("clicked", interaction.clicked)?;
            Ok(result)
        })
    })?)?;
    env.set("UI", ui)?;
    Ok(())
}
