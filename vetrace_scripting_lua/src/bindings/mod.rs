use std::collections::BTreeMap;

use mlua::{AnyUserData, Lua, MetaMethod, Table, UserData, UserDataMethods, Value};
use vetrace_core::{DynamicValue, Entity, FieldPath, InputState, Transform};
use vetrace_physics::{raycast_colliders, PhysicsActorExt};
use vetrace_render::{RenderSettings, ScreenSpaceRect};
use vetrace_ui::UIButton;

use crate::context::{
    allocate_spawn_request, queue_command, resolve_entity_target, with_context, LuaCommand,
    LuaEntityTarget,
};
use crate::input::LuaInputMap;

mod dynamic_values;
mod entity_component_api;
mod entity_helpers;
mod game_api;
mod input;
mod modules_api;
mod script_values;
mod transforms;
mod vectors;

pub(crate) use entity_component_api::install_entity_component_api;
pub(crate) use game_api::install_game_api;

use dynamic_values::{
    append_text_path, child_path_for_lua_key, dynamic_to_lua_table, is_dynamic_container,
    lua_key_is_value, lua_to_dynamic, read_component_value, set_component_root,
    set_component_value,
};
use entity_helpers::{
    add_entity_tag, component_ids, component_proxy, find_all_entities_by_tag,
    find_entity_by_name, find_entity_by_tag, has_component, queue_add_component, queue_audio,
    queue_remove_component, resolve_entity_now, set_entity_name, spawn_pending, with_live_actor,
};
use input::{
    input_action_down, input_action_pressed, input_action_released, input_key_down,
    input_key_pressed, input_key_released, with_input,
};
use modules_api::install_modules_api;
use script_values::{display_lua_value, lua_number, lua_to_script_value, print_lua_value};
use transforms::{
    ensure_transform, read_transform, rotation, scale, set_rotation, set_scale,
    set_translation, translate, translation,
};
use vectors::{
    normalized_quat, parse_raycast_arguments, parse_vec3_argument,
    table_to_vec3, value_to_f32, vec3_to_lua_table,
};


/// Compatibility object passed to legacy scripts.
///
/// It deliberately stores no engine pointer. Every operation resolves through
/// the callback-scoped Lua context and therefore becomes invalid as soon as the
/// callback returns.
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineHandle;

impl UserData for EngineHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("spawn", |lua, _this, name: Option<String>| {
            spawn_pending(lua, name.unwrap_or_else(|| "Actor".to_owned()))
        });
        methods.add_method("spawn_named", |lua, _this, name: String| spawn_pending(lua, name));
        methods.add_method("delete_entity", |_, _this, entity: AnyUserData| {
            let proxy = entity.borrow::<EntityProxy>()?;
            if let Some(entity) = resolve_entity_now(proxy.target)? {
                queue_command(LuaCommand::Destroy(entity))?;
            }
            Ok(())
        });
        methods.add_method("find_entity_by_name", |lua, _this, name: String| {
            find_entity_by_name(lua, &name)
        });
        methods.add_method("clear_scene", |_, _this, ()| queue_command(LuaCommand::ClearScene));
        methods.add_method("entity_count", |_, _this, ()| {
            with_context(|engine, _, _, _, _, _| Ok(engine.raw_world().entities().count() as u64))
        });
        methods.add_method("stop", |_, _this, ()| queue_command(LuaCommand::Stop));
        methods.add_method("print", |_, _this, value: Value| {
            print_lua_value(value);
            Ok(())
        });
    }
}

/// Compatibility input object passed to legacy scripts.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputProxy;

impl InputProxy {
    pub fn new() -> Self { Self }
}

impl UserData for InputProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("is_key_down", |_, _this, key: String| input_key_down(&key));
        methods.add_method("was_key_pressed", |_, _this, key: String| input_key_pressed(&key));
        methods.add_method("was_key_released", |_, _this, key: String| input_key_released(&key));
        methods.add_method("is_mouse_button_down", |_, _this, button: String| {
            with_input(|input| input.is_mouse_button_down(&button))
        });
        methods.add_method("mouse_position", |_, _this, ()| {
            with_context(|engine, _, _, _, _, _| {
                Ok(engine
                    .get_resource::<InputState>()
                    .map(InputState::mouse_position)
                    .unwrap_or((0.0, 0.0)))
            })
        });
        methods.add_method("action_down", |_, _this, action: String| input_action_down(&action));
        methods.add_method("action_pressed", |_, _this, action: String| input_action_pressed(&action));
        methods.add_method("action_released", |_, _this, action: String| input_action_released(&action));
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntityProxy {
    target: LuaEntityTarget,
}

impl EntityProxy {
    pub fn live(entity: Entity) -> Self { Self { target: LuaEntityTarget::Live(entity) } }
    pub(crate) fn pending(request: u64) -> Self { Self { target: LuaEntityTarget::Pending(request) } }
    pub fn entity(&self) -> Entity { self.live_entity().unwrap_or(Entity::INVALID) }
    pub(crate) fn live_entity(&self) -> Option<Entity> {
        match self.target {
            LuaEntityTarget::Live(entity) => Some(entity),
            LuaEntityTarget::Pending(_) => None,
        }
    }
}

impl UserData for EntityProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| match this.target {
            LuaEntityTarget::Live(entity) => Ok(Some(entity.0)),
            LuaEntityTarget::Pending(_) => Ok(None),
        });
        methods.add_method("is_pending", |_, this, ()| Ok(matches!(this.target, LuaEntityTarget::Pending(_))));
        methods.add_method("is_spawned", |_, this, ()| Ok(resolve_entity_now(this.target)?.is_some()));
        methods.add_method("is_alive", |_, this, ()| {
            let Some(entity) = resolve_entity_now(this.target)? else {
                return Ok(matches!(this.target, LuaEntityTarget::Pending(_)));
            };
            with_context(|engine, _, _, _, _, _| Ok(engine.actor(entity).is_some()))
        });
        methods.add_method("get_name", |_, this, ()| match this.target {
            LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
                Ok(engine.actor(entity).and_then(|actor| actor.name(engine)).map(ToOwned::to_owned))
            }),
            LuaEntityTarget::Pending(_) => Ok(None),
        });
        methods.add_method("set_name", |_, this, name: String| set_entity_name(this.target, name));
        methods.add_method("has_tag", |_, this, tag: String| match this.target {
            LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
                Ok(engine.actor(entity).is_some_and(|actor| actor.has_tag(engine, &tag)))
            }),
            LuaEntityTarget::Pending(_) => Ok(false),
        });
        methods.add_method("add_tag", |_, this, tag: String| add_entity_tag(this.target, tag));
        methods.add_method("remove_tag", |_, this, tag: String| match this.target {
            LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
                if let Some(actor) = engine.actor(entity) {
                    actor.remove_tag(engine, &tag).map_err(mlua::Error::external)?;
                }
                Ok(())
            }),
            LuaEntityTarget::Pending(_) => Ok(()),
        });
        methods.add_method("translation", |_, this, ()| translation(this.target));
        methods.add_method("rotation", |_, this, ()| rotation(this.target));
        methods.add_method("set_rotation_quat", |_, this, value: (f32, f32, f32, f32)| {
            set_rotation(this.target, normalized_quat(value.0, value.1, value.2, value.3))
        });
        methods.add_method("set_rotation_euler_degrees", |_, this, value: (f32, f32, f32)| {
            set_rotation(this.target, glam::Quat::from_euler(
                glam::EulerRot::YXZ,
                value.1.to_radians(),
                value.0.to_radians(),
                value.2.to_radians(),
            ))
        });
        methods.add_method("set_translation", |_, this, value: (f32, f32, f32)| {
            set_translation(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });
        methods.add_method("translate", |_, this, value: (f32, f32, f32)| {
            translate(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });
        methods.add_method("scale", |_, this, ()| scale(this.target));
        methods.add_method("set_scale", |_, this, value: (f32, f32, f32)| {
            set_scale(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });
        methods.add_method("has_component", |_, this, component: String| {
            has_component(this.target, &component)
        });
        methods.add_method("get_component", |lua, this, component: String| {
            component_proxy(lua, this.target, &component, true)
        });
        methods.add_method("add_component", |_, this, (component, value): (String, Option<Value>)| {
            queue_add_component(this.target, component, value)
        });
        methods.add_method("remove_component", |_, this, component: String| {
            queue_remove_component(this.target, component)
        });
        methods.add_method("component_ids", |lua, this, ()| component_ids(lua, this.target));
        methods.add_method("destroy", |_, this, ()| {
            if let Some(entity) = resolve_entity_now(this.target)? {
                queue_command(LuaCommand::Destroy(entity))?;
            }
            Ok(())
        });

        methods.add_meta_method(MetaMethod::Index, |lua, this, key: String| match key.as_str() {
            "id" => match this.target {
                LuaEntityTarget::Live(entity) => Ok(Value::Integer(entity.0 as i64)),
                LuaEntityTarget::Pending(request) => {
                    let pending = format!("pending:{request}");
                    Ok(Value::String(lua.create_string(pending.as_str())?))
                },
            },
            "name" => match this.target {
                LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
                    match engine.actor(entity).and_then(|actor| actor.name(engine)) {
                        Some(name) => Ok(Value::String(lua.create_string(name)?)),
                        None => Ok(Value::Nil),
                    }
                }),
                LuaEntityTarget::Pending(_) => Ok(Value::Nil),
            },
            "tags" => match this.target {
                LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
                    let table = lua.create_table()?;
                    if let Some(metadata) = engine.actor(entity).and_then(|actor| actor.metadata(engine)) {
                        for (index, tag) in metadata.tags.iter().enumerate() {
                            table.set(index + 1, tag.as_str())?;
                        }
                    }
                    Ok(Value::Table(table))
                }),
                LuaEntityTarget::Pending(_) => Ok(Value::Table(lua.create_table()?)),
            },
            "transform" => Ok(Value::UserData(lua.create_userdata(TransformProxy { target: this.target })?)),
            "components" => Ok(Value::UserData(lua.create_userdata(ComponentCollectionProxy { target: this.target })?)),
            _ => Ok(Value::Nil),
        });
    }
}


#[derive(Clone, Copy, Debug)]
pub struct ComponentCollectionProxy {
    target: LuaEntityTarget,
}

impl ComponentCollectionProxy {
    pub fn live(entity: Entity) -> Self { Self { target: LuaEntityTarget::Live(entity) } }
}

impl UserData for ComponentCollectionProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("has", |_, this, component: String| has_component(this.target, &component));
        methods.add_method("get", |lua, this, component: String| {
            component_proxy(lua, this.target, &component, true)
        });
        methods.add_method("ids", |lua, this, ()| component_ids(lua, this.target));
        methods.add_method("add", |_, this, (component, value): (String, Option<Value>)| {
            queue_add_component(this.target, component, value)
        });
        methods.add_method("remove", |_, this, component: String| {
            queue_remove_component(this.target, component)
        });

        methods.add_meta_method(MetaMethod::Index, |lua, this, key: String| {
            component_proxy(lua, this.target, &key, true)
        });
        methods.add_meta_method_mut(MetaMethod::NewIndex, |_, this, (key, value): (String, Value)| {
            if matches!(value, Value::Nil) {
                queue_remove_component(this.target, key)
            } else if has_component(this.target, &key)? {
                set_component_root(this.target, &key, lua_to_dynamic(value)?)
            } else {
                queue_add_component(this.target, key, Some(value))
            }
        });
    }
}

#[derive(Clone, Debug)]
pub struct DynamicComponentProxy {
    target: LuaEntityTarget,
    component: String,
    path: FieldPath,
}

impl DynamicComponentProxy {
    fn root(target: LuaEntityTarget, component: String) -> Self {
        Self { target, component, path: FieldPath::root() }
    }

    fn nested(&self, path: FieldPath) -> Self {
        Self { target: self.target, component: self.component.clone(), path }
    }
}

impl UserData for DynamicComponentProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("component_id", |_, this, ()| Ok(this.component.clone()));
        methods.add_method("path", |_, this, ()| Ok(this.path.to_string()));
        methods.add_method("get", |lua, this, path: Option<String>| {
            let path = append_text_path(&this.path, path.as_deref().unwrap_or(""))?;
            let value = read_component_value(this.target, &this.component, &path)?;
            dynamic_to_lua_table(lua, &value)
        });
        methods.add_method("set", |_, this, (path, value): (String, Value)| {
            let path = append_text_path(&this.path, &path)?;
            set_component_value(this.target, &this.component, &path, lua_to_dynamic(value)?)
        });
        methods.add_method("to_table", |lua, this, ()| {
            let value = read_component_value(this.target, &this.component, &this.path)?;
            dynamic_to_lua_table(lua, &value)
        });
        methods.add_method("schema", |lua, this, ()| {
            with_live_actor(this.target, |engine, actor| {
                let schema = engine.component_schema(Some(actor), &this.component)
                    .map_err(mlua::Error::external)?;
                let dynamic = DynamicValue::from_serialize(&schema).map_err(mlua::Error::external)?;
                dynamic_to_lua_table(lua, &dynamic)
            })
        });

        methods.add_meta_method(MetaMethod::Index, |lua, this, key: Value| {
            let current = read_component_value(this.target, &this.component, &this.path)?;
            if lua_key_is_value(&key) && !is_dynamic_container(&current) {
                return dynamic_to_lua_table(lua, &current);
            }
            let child_path = child_path_for_lua_key(&this.path, &current, key)?;
            let child = read_component_value(this.target, &this.component, &child_path)?;
            match child {
                DynamicValue::Object(_) | DynamicValue::Array(_) => {
                    Ok(Value::UserData(lua.create_userdata(this.nested(child_path))?))
                }
                scalar => dynamic_to_lua_table(lua, &scalar),
            }
        });
        methods.add_meta_method_mut(MetaMethod::NewIndex, |_, this, (key, value): (Value, Value)| {
            let current = read_component_value(this.target, &this.component, &this.path)?;
            let child_path = if lua_key_is_value(&key) && !is_dynamic_container(&current) {
                this.path.clone()
            } else {
                child_path_for_lua_key(&this.path, &current, key)?
            };
            set_component_value(this.target, &this.component, &child_path, lua_to_dynamic(value)?)
        });
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TransformProxy {
    target: LuaEntityTarget,
}

impl TransformProxy {
    pub fn live(entity: Entity) -> Self { Self { target: LuaEntityTarget::Live(entity) } }
}

impl UserData for TransformProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("translation", |_, this, ()| translation(this.target));
        methods.add_method("rotation", |_, this, ()| rotation(this.target));
        methods.add_method("set_rotation_quat", |_, this, value: (f32, f32, f32, f32)| {
            set_rotation(this.target, normalized_quat(value.0, value.1, value.2, value.3))
        });
        methods.add_method("set_rotation_euler_degrees", |_, this, value: (f32, f32, f32)| {
            set_rotation(this.target, glam::Quat::from_euler(
                glam::EulerRot::YXZ,
                value.1.to_radians(),
                value.0.to_radians(),
                value.2.to_radians(),
            ))
        });
        methods.add_method("set_translation", |_, this, value: (f32, f32, f32)| {
            set_translation(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });
        methods.add_method("translate", |_, this, value: (f32, f32, f32)| {
            translate(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });
        methods.add_method("scale", |_, this, ()| scale(this.target));
        methods.add_method("set_scale", |_, this, value: (f32, f32, f32)| {
            set_scale(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });
        methods.add_method("translate_xyz", |_, this, value: (f32, f32, f32)| {
            translate(this.target, glam::Vec3::new(value.0, value.1, value.2))
        });

        methods.add_meta_method(MetaMethod::Index, |_, this, field: String| {
            let transform = read_transform(this.target)?;
            match field.as_str() {
                "x" => Ok(Value::Number(transform.translation.x as f64)),
                "y" => Ok(Value::Number(transform.translation.y as f64)),
                "z" => Ok(Value::Number(transform.translation.z as f64)),
                "sx" => Ok(Value::Number(transform.scale.x as f64)),
                "sy" => Ok(Value::Number(transform.scale.y as f64)),
                "sz" => Ok(Value::Number(transform.scale.z as f64)),
                _ => Ok(Value::Nil),
            }
        });
        methods.add_meta_method_mut(MetaMethod::NewIndex, |_, this, (field, value): (String, Value)| {
            let number = lua_number(value);
            let Some(number) = number else { return Ok(()); };
            match this.target {
                LuaEntityTarget::Live(entity) => with_context(|engine, _, _, _, _, _| {
                    let transform = ensure_transform(engine, entity)?;
                    match field.as_str() {
                        "x" => transform.translation.x = number,
                        "y" => transform.translation.y = number,
                        "z" => transform.translation.z = number,
                        "sx" => transform.scale.x = number,
                        "sy" => transform.scale.y = number,
                        "sz" => transform.scale.z = number,
                        _ => {}
                    }
                    Ok(())
                }),
                LuaEntityTarget::Pending(request) => {
                    let mut transform = read_transform(this.target)?;
                    match field.as_str() {
                        "x" => transform.translation.x = number,
                        "y" => transform.translation.y = number,
                        "z" => transform.translation.z = number,
                        "sx" => transform.scale.x = number,
                        "sy" => transform.scale.y = number,
                        "sz" => transform.scale.z = number,
                        _ => return Ok(()),
                    }
                    queue_command(LuaCommand::SetPendingTranslation { request, value: transform.translation })?;
                    queue_command(LuaCommand::SetPendingScale { request, value: transform.scale })
                }
            }
        });
    }
}
