use super::*;

pub(super) fn install_rendering_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let rendering = lua.create_table()?;
    rendering.set("get", lua.create_function(|lua, key: String| {
        with_context(|engine, _, _, _, _, _| {
            let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
            render_setting_to_lua(lua, &settings, &key)
        })
    })?)?;
    rendering.set("set", lua.create_function(|_, (key, value): (String, Value)| {
        with_context(|engine, _, _, _, _, _| {
            let settings = engine
                .get_resource_mut::<RenderSettings>()
                .ok_or_else(|| mlua::Error::external("render settings are unavailable"))?;
            set_render_setting(settings, &key, value)
        })
    })?)?;
    env.set("Rendering", rendering)
}

pub(super) fn render_setting_to_lua(lua: &Lua, settings: &RenderSettings, key: &str) -> mlua::Result<Value> {
    Ok(match key {
        "vsync" => Value::Boolean(matches!(settings.present_mode, PresentModePreference::Vsync | PresentModePreference::Fifo)),
        "present_mode" => Value::String(lua.create_string(match settings.present_mode {
            PresentModePreference::Vsync => "vsync",
            PresentModePreference::LowLatency => "low_latency",
            PresentModePreference::Immediate => "immediate",
            PresentModePreference::Mailbox => "mailbox",
            PresentModePreference::Fifo => "fifo",
        })?),
        "anti_aliasing" => Value::String(lua.create_string(match settings.anti_aliasing_mode {
            AntiAliasingMode::Off => "off",
            AntiAliasingMode::Fxaa => "fxaa",
        })?),
        "ambient_occlusion" => Value::String(lua.create_string(match settings.ambient_occlusion_mode {
            AmbientOcclusionMode::Off => "off",
            AmbientOcclusionMode::Ssao => "ssao",
        })?),
        "shadow_map_size" => Value::Integer(settings.shadow_map_size.into()),
        "shadow_max_distance" => Value::Number(settings.shadow_max_distance.into()),
        "shadow_soft_radius" => Value::Number(settings.shadow_soft_radius.into()),
        "shadow_filter" => Value::String(lua.create_string(match settings.shadow_filter_mode {
            ShadowFilterMode::Hard => "hard",
            ShadowFilterMode::Pcf => "pcf",
            ShadowFilterMode::Pcss => "pcss",
            ShadowFilterMode::EvsmBlurred => "evsm_blurred",
        })?),
        "cursor_visible" => Value::Boolean(settings.cursor_visible),
        "cursor_grab" => Value::Boolean(settings.cursor_grab),
        "draw_bounds" => Value::Boolean(settings.draw_bounds),
        "draw_names" => Value::Boolean(settings.draw_names),
        _ => Value::Nil,
    })
}

pub(super) fn set_render_setting(settings: &mut RenderSettings, key: &str, value: Value) -> mlua::Result<()> {
    match key {
        "vsync" => {
            settings.present_mode = if expect_bool(value, key)? {
                PresentModePreference::Vsync
            } else {
                PresentModePreference::LowLatency
            };
        }
        "present_mode" => {
            settings.present_mode = match expect_string(value, key)?.as_str() {
                "vsync" => PresentModePreference::Vsync,
                "low_latency" => PresentModePreference::LowLatency,
                "immediate" => PresentModePreference::Immediate,
                "mailbox" => PresentModePreference::Mailbox,
                "fifo" => PresentModePreference::Fifo,
                other => return Err(mlua::Error::external(format!("unsupported present mode '{other}'"))),
            };
        }
        "anti_aliasing" => {
            settings.anti_aliasing_mode = match expect_string(value, key)?.as_str() {
                "off" => AntiAliasingMode::Off,
                "fxaa" => AntiAliasingMode::Fxaa,
                other => return Err(mlua::Error::external(format!("unsupported anti-aliasing mode '{other}'"))),
            };
        }
        "ambient_occlusion" => {
            settings.ambient_occlusion_mode = match expect_string(value, key)?.as_str() {
                "off" => AmbientOcclusionMode::Off,
                "ssao" => AmbientOcclusionMode::Ssao,
                other => return Err(mlua::Error::external(format!("unsupported ambient-occlusion mode '{other}'"))),
            };
        }
        "shadow_map_size" => settings.shadow_map_size = expect_number(value, key)?.round().clamp(128.0, 8192.0) as u32,
        "shadow_max_distance" => settings.shadow_max_distance = expect_number(value, key)?.clamp(0.0, 20_000.0),
        "shadow_soft_radius" => settings.shadow_soft_radius = expect_number(value, key)?.clamp(0.0, 32.0),
        "shadow_filter" => {
            settings.shadow_filter_mode = match expect_string(value, key)?.as_str() {
                "hard" => ShadowFilterMode::Hard,
                "pcf" => ShadowFilterMode::Pcf,
                "pcss" => ShadowFilterMode::Pcss,
                "evsm_blurred" => ShadowFilterMode::EvsmBlurred,
                other => return Err(mlua::Error::external(format!("unsupported shadow filter '{other}'"))),
            };
        }
        "cursor_visible" => settings.cursor_visible = expect_bool(value, key)?,
        "cursor_grab" => settings.cursor_grab = expect_bool(value, key)?,
        "draw_bounds" => settings.draw_bounds = expect_bool(value, key)?,
        "draw_names" => settings.draw_names = expect_bool(value, key)?,
        other => return Err(mlua::Error::external(format!("unknown render setting '{other}'"))),
    }
    Ok(())
}
