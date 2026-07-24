use super::*;

pub(super) fn vec3_to_lua_table(lua: &Lua, value: glam::Vec3) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(1, value.x)?;
    table.set(2, value.y)?;
    table.set(3, value.z)?;
    table.set("x", value.x)?;
    table.set("y", value.y)?;
    table.set("z", value.z)?;
    Ok(table)
}

pub(super) fn parse_vec3_argument(
    x_or_vector: Value,
    y: Option<f32>,
    z: Option<f32>,
    function_name: &str,
) -> mlua::Result<glam::Vec3> {
    match x_or_vector {
        Value::Table(table) if y.is_none() && z.is_none() => table_to_vec3(table, function_name),
        Value::Integer(x) => {
            let (Some(y), Some(z)) = (y, z) else {
                return Err(mlua::Error::external(format!(
                    "{function_name} expects either a vec3 table or three numeric components"
                )));
            };
            Ok(glam::Vec3::new(x as f32, y, z))
        }
        Value::Number(x) => {
            let (Some(y), Some(z)) = (y, z) else {
                return Err(mlua::Error::external(format!(
                    "{function_name} expects either a vec3 table or three numeric components"
                )));
            };
            Ok(glam::Vec3::new(x as f32, y, z))
        }
        other => Err(mlua::Error::external(format!(
            "{function_name} expects either a vec3 table or three numeric components; got {other:?}"
        ))),
    }
}

pub(super) fn table_to_vec3(table: Table, function_name: &str) -> mlua::Result<glam::Vec3> {
    let x = match table.get::<Option<f32>>("x")? {
        Some(value) => Some(value),
        None => table.get::<Option<f32>>(1)?,
    };
    let y = match table.get::<Option<f32>>("y")? {
        Some(value) => Some(value),
        None => table.get::<Option<f32>>(2)?,
    };
    let z = match table.get::<Option<f32>>("z")? {
        Some(value) => Some(value),
        None => table.get::<Option<f32>>(3)?,
    };
    match (x, y, z) {
        (Some(x), Some(y), Some(z)) => Ok(glam::Vec3::new(x, y, z)),
        _ => Err(mlua::Error::external(format!(
            "{function_name} vec3 table must contain x/y/z or indexes 1/2/3"
        ))),
    }
}

pub(super) fn value_to_f32(value: Value, argument_name: &str) -> mlua::Result<f32> {
    match value {
        Value::Integer(value) => Ok(value as f32),
        Value::Number(value) => Ok(value as f32),
        other => Err(mlua::Error::external(format!(
            "{argument_name} must be numeric; got {other:?}"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn parse_raycast_arguments(
    a: Value,
    b: Value,
    c: Option<Value>,
    d: Option<Value>,
    e: Option<Value>,
    f: Option<Value>,
    g: Option<Value>,
) -> mlua::Result<(glam::Vec3, glam::Vec3, Option<f32>)> {
    if matches!(&a, Value::Table(_)) && matches!(&b, Value::Table(_)) {
        if d.is_some() || e.is_some() || f.is_some() || g.is_some() {
            return Err(mlua::Error::external(
                "Physics.raycast(origin, direction, max_distance) received too many arguments",
            ));
        }
        let Value::Table(origin) = a else { unreachable!() };
        let Value::Table(direction) = b else { unreachable!() };
        let origin = table_to_vec3(origin, "Physics.raycast origin")?;
        let direction = table_to_vec3(direction, "Physics.raycast direction")?;
        let max_distance = c
            .map(|value| value_to_f32(value, "Physics.raycast max_distance"))
            .transpose()?;
        return Ok((origin, direction, max_distance));
    }

    let Some(c) = c else {
        return Err(mlua::Error::external(
            "Physics.raycast expects origin and direction vec3 tables, or six numeric components",
        ));
    };
    let (Some(d), Some(e), Some(f)) = (d, e, f) else {
        return Err(mlua::Error::external(
            "Physics.raycast numeric form is Physics.raycast(ox, oy, oz, dx, dy, dz[, max_distance])",
        ));
    };
    let origin = glam::Vec3::new(
        value_to_f32(a, "Physics.raycast origin.x")?,
        value_to_f32(b, "Physics.raycast origin.y")?,
        value_to_f32(c, "Physics.raycast origin.z")?,
    );
    let direction = glam::Vec3::new(
        value_to_f32(d, "Physics.raycast direction.x")?,
        value_to_f32(e, "Physics.raycast direction.y")?,
        value_to_f32(f, "Physics.raycast direction.z")?,
    );
    let max_distance = g
        .map(|value| value_to_f32(value, "Physics.raycast max_distance"))
        .transpose()?;
    Ok((origin, direction, max_distance))
}

pub(super) fn normalized_quat(x: f32, y: f32, z: f32, w: f32) -> glam::Quat {
    let value = glam::Quat::from_xyzw(x, y, z, w);
    if !value.is_finite() || value.length_squared() <= f32::EPSILON {
        glam::Quat::IDENTITY
    } else {
        value.normalize()
    }
}
