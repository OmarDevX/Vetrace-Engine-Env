use super::*;

pub(super) fn install_camera_api(lua: &Lua, env: &Table) -> mlua::Result<()> {
    let camera = lua.create_table()?;
    camera.set("position", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            let value = engine.get_resource::<Camera>().cloned().unwrap_or_default().position;
            Ok((value.x, value.y, value.z))
        })
    })?)?;
    camera.set("target", lua.create_function(|_, ()| {
        with_context(|engine, _, _, _, _, _| {
            let value = engine.get_resource::<Camera>().cloned().unwrap_or_default().target;
            Ok((value.x, value.y, value.z))
        })
    })?)?;
    camera.set("set_position", lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(camera) = engine.get_resource_mut::<Camera>() {
                camera.position = glam::Vec3::new(x, y, z);
            }
            Ok(())
        })
    })?)?;
    camera.set("set_target", lua.create_function(|_, (x, y, z): (f32, f32, f32)| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(camera) = engine.get_resource_mut::<Camera>() {
                camera.target = glam::Vec3::new(x, y, z);
            }
            Ok(())
        })
    })?)?;
    camera.set("set_fov_degrees", lua.create_function(|_, degrees: f32| {
        with_context(|engine, _, _, _, _, _| {
            if let Some(camera) = engine.get_resource_mut::<Camera>() {
                camera.fov_y_radians = degrees.clamp(20.0, 150.0).to_radians();
            }
            Ok(())
        })
    })?)?;
    env.set("Camera", camera)
}
