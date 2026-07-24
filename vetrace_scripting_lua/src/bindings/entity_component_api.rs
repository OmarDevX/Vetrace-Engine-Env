use super::*;

pub(crate) fn install_entity_component_api(lua: &Lua, table: &Table, entity: Entity) -> mlua::Result<()> {
    let target = LuaEntityTarget::Live(entity);
    table.set("components", lua.create_userdata(ComponentCollectionProxy::live(entity))?)?;
    table.set("has_component", lua.create_function(move |_, (_self, component): (Table, String)| {
        has_component(target, &component)
    })?)?;
    table.set("get_component", lua.create_function(move |lua, (_self, component): (Table, String)| {
        component_proxy(lua, target, &component, true)
    })?)?;
    table.set("add_component", lua.create_function(move |_, (_self, component, value): (Table, String, Option<Value>)| {
        queue_add_component(target, component, value)
    })?)?;
    table.set("remove_component", lua.create_function(move |_, (_self, component): (Table, String)| {
        queue_remove_component(target, component)
    })?)?;
    table.set("component_ids", lua.create_function(move |lua, _self: Table| {
        component_ids(lua, target)
    })?)?;
    Ok(())
}
