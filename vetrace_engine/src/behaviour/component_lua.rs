use std::any::TypeId;
use std::path::Path;

use mlua::{Lua, Function, Value};

use crate::ecs::Behaviour;
use crate::inspector::Inspectable;
use crate::engine::engine::Engine;
use super::script::EngineHandle;

pub struct LuaComponentBehaviour {
    pub component: String,
    lua: Lua,
}

impl LuaComponentBehaviour {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();
        let script = std::fs::read_to_string(path)?;
        lua.load(&script).set_name(path.to_str().unwrap_or("script")).exec()?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .replace("Behaviour", "");
        Ok(Self { component: name, lua })
    }

    fn call_fn(&self, name: &str, engine_ptr: *mut Engine, comp: &mut dyn Inspectable, delta: Option<f32>) {
        if let Ok(func) = self.lua.globals().get::<Function>(name) {
            let table = self.lua.create_table().unwrap();
            for field in comp.exported_fields_mut() {
                unsafe {
                    let val = if field.type_id == TypeId::of::<f32>() {
                        Value::Number(*(field.value as *mut f32) as f64)
                    } else if field.type_id == TypeId::of::<i32>() {
                        Value::Integer(*(field.value as *mut i32) as i64)
                    } else if field.type_id == TypeId::of::<u32>() {
                        Value::Integer(*(field.value as *mut u32) as i64)
                    } else if field.type_id == TypeId::of::<bool>() {
                        Value::Boolean(*(field.value as *mut bool))
                    } else {
                        continue;
                    };
                    table.set(field.name, val).unwrap();
                }
            }

            if let Some(d) = delta {
                let _ : () = func.call((EngineHandle(engine_ptr), table.clone(), d)).unwrap_or(());
            } else {
                let _ : () = func.call((EngineHandle(engine_ptr), table.clone())).unwrap_or(());
            }

            for field in comp.exported_fields_mut() {
                if let Ok(val) = table.get::<Value>(field.name) {
                    unsafe {
                        match val {
                            Value::Number(n) => {
                                if field.type_id == TypeId::of::<f32>() {
                                    *(field.value as *mut f32) = n as f32;
                                } else if field.type_id == TypeId::of::<i32>() {
                                    *(field.value as *mut i32) = n as i32;
                                } else if field.type_id == TypeId::of::<u32>() {
                                    *(field.value as *mut u32) = n as u32;
                                }
                            }
                            Value::Integer(i) => {
                                if field.type_id == TypeId::of::<i32>() {
                                    *(field.value as *mut i32) = i as i32;
                                } else if field.type_id == TypeId::of::<u32>() {
                                    *(field.value as *mut u32) = i as u32;
                                }
                            }
                            Value::Boolean(b) => {
                                if field.type_id == TypeId::of::<bool>() {
                                    *(field.value as *mut bool) = b;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

impl Behaviour for LuaComponentBehaviour {
    fn update(&mut self, engine: &mut Engine, delta_time: f32) {
        let checker = if let Some(c) = engine.component_checkers.get(&self.component) {
            c.clone()
        } else {
            return;
        };
        let engine_ptr = engine as *mut Engine;
        for entity in engine.world.entities().to_vec() {
            if checker(&engine.world, entity) {
                if let Some(comp) = engine.access_component_mut(entity, &self.component) {
                    self.call_fn("update", engine_ptr, comp, Some(delta_time));
                }
            }
        }
    }

    fn start(&mut self, engine: &mut Engine) {
        let checker = if let Some(c) = engine.component_checkers.get(&self.component) {
            c.clone()
        } else {
            return;
        };
        let engine_ptr = engine as *mut Engine;
        for entity in engine.world.entities().to_vec() {
            if checker(&engine.world, entity) {
                if let Some(comp) = engine.access_component_mut(entity, &self.component) {
                    self.call_fn("start", engine_ptr, comp, None);
                }
            }
        }
    }
}

