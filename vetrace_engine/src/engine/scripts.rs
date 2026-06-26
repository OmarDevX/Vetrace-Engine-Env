use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::fs;
use crate::behaviour::script::ScriptBehaviour;
use crate::behaviour::component_lua::LuaComponentBehaviour;
use crate::components::components::ScriptComponent;
use crate::components::generated::{GeneratedSpec, GeneratedStorage, FieldType};
use crate::ecs::behaviour::Behaviour;
use crate::inspector::Inspectable;
use super::Engine;

impl Engine {
    pub fn start_script_components(&mut self) {
        for entity in self.world.entities().to_vec() {
            if self.started_scripts.contains(&entity) { continue; }
            let script_name = if let Some(comp) = self.get_component_mut_entity::<ScriptComponent>(entity) {
                Some(comp.script.clone())
            } else { None };
            if let Some(name) = script_name {
                let script_ptr = self
                    .script_library
                    .get_mut(&name)
                    .map(|s| s as *mut ScriptBehaviour);
                // Insert before calling start to avoid re-entrant spawning from
                // scripts triggering another start on the same entity.
                self.started_scripts.insert(entity);
                if let Some(ptr) = script_ptr {
                    unsafe { (*ptr).start(self, entity.0); }
                }
            }
        }
    }

    pub fn update_script_components(&mut self, delta: f32) {
        for entity in self.world.entities().to_vec() {
            let script_name = if let Some(comp) = self.get_component_mut_entity::<ScriptComponent>(entity) {
                Some(comp.script.clone())
            } else { None };
            if let Some(name) = script_name {
                let script_ptr = self.script_library.get_mut(&name).map(|s| s as *mut ScriptBehaviour);
                if let Some(ptr) = script_ptr { unsafe { (*ptr).update(self, entity.0, delta); } }
            }
        }
    }

    pub fn start_component_behaviours(&mut self) {
        let mut ptrs: Vec<*mut LuaComponentBehaviour> = self.component_behaviours.values_mut().map(|b| b as *mut _).collect();
        for ptr in ptrs.iter_mut() { unsafe { (*(*ptr)).start(self); } }
    }

    pub fn update_component_behaviours(&mut self, delta: f32) {
        let mut ptrs: Vec<*mut LuaComponentBehaviour> = self.component_behaviours.values_mut().map(|b| b as *mut _).collect();
        for ptr in ptrs.iter_mut() { unsafe { (*(*ptr)).update(self, delta); } }
    }

    pub fn reload_scripts(&mut self) {
        self.script_library.clear();
        self.started_scripts.clear();
        let dir = std::path::Path::new("generated");
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                    if let Ok(beh) = ScriptBehaviour::from_file(&path) {
                        self.script_library.insert(beh.name.clone(), beh);
                    }
                }
            }
        }
        self.start_script_components();
        self.reload_component_behaviours();
    }

    pub fn reload_component_behaviours(&mut self) {
        self.component_behaviours.clear();
        let dir = std::path::Path::new("generated/behaviours");
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("lua") {
                    if let Ok(beh) = LuaComponentBehaviour::from_file(&path) {
                        self.component_behaviours.insert(beh.component.clone(), beh);
                    }
                }
            }
        }
        self.start_component_behaviours();
        self.update_generated_components();
    }

    pub fn update_generated_components(&mut self) {
        self.generated_components.clear();
        self.generated_specs.clear();
        let mut dirs = vec![std::path::PathBuf::from("generated/components")];
        if let Ok(ws) = std::env::var("CARGO_WORKSPACE_DIR") { dirs.push(std::path::Path::new(&ws).join("generated/components")); }
        for dir in dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            if !self.generated_components.contains(&name.to_string()) {
                                self.generated_components.push(name.to_string());
                            }
                            let spec_path = path.with_extension("spec");
                            if let Ok(text) = std::fs::read_to_string(&spec_path) {
                                let mut fields = Vec::new();
                                for line in text.lines() {
                                    let mut parts = line.split_whitespace();
                                    if let (Some(fname), Some(ftype)) = (parts.next(), parts.next()) {
                                        let ty = match ftype { "f32" => FieldType::F32, "i32" => FieldType::I32, "bool" => FieldType::Bool, _ => continue };
                                        let leaked: &'static mut str = Box::leak(fname.to_string().into_boxed_str());
                                        let name_static: &'static str = leaked;
                                        fields.push((name_static, ty));
                                    }
                                }
                                if !fields.is_empty() { self.generated_specs.insert(name.to_string(), GeneratedSpec { fields }); }
                            }
                            self.register_runtime_generated(name);
                        }
                    }
                }
            }
        }
    }

    fn register_runtime_generated(&mut self, name: &str) {
        if self.component_adders.contains_key(name) { return; }
        let name_add = name.to_string();
        self.component_adders.insert(name_add.clone(), Rc::new(move |engine, entity| { engine.add_generated_component(entity, &name_add); }));
        let name_remove = name.to_string();
        self.component_removers.insert(name_remove.clone(), Rc::new(move |engine, entity| { engine.remove_generated_component(entity, &name_remove); }));
        let name_edit = name.to_string();
        self.component_editors.insert(name_edit.clone(), Rc::new(move |engine, entity, ui| { if let Some(c) = engine.get_generated_component_mut(entity, &name_edit) { c.draw_ui(ui); } }));
        let name_check = name.to_string();
        self.component_checkers.insert(name_check.clone(), Rc::new(move |world, entity| {
            world.get::<GeneratedStorage>(entity).map(|s| s.components.contains_key(&name_check)).unwrap_or(false)
        }));
    }
}
