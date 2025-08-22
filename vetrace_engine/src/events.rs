use std::clone::Clone;
use mlua::{Function, UserData, UserDataMethods, Value};

pub struct Event<T: Clone> {
    subscribers: Vec<Box<dyn FnMut(T)>>,
}

impl<T: Clone> Event<T> {
    pub fn new() -> Self {
        Self { subscribers: Vec::new() }
    }

    pub fn subscribe<F>(&mut self, f: F)
    where
        F: FnMut(T) + 'static,
    {
        self.subscribers.push(Box::new(f));
    }

    pub fn emit(&mut self, data: T) {
        for sub in &mut self.subscribers {
            sub(data.clone());
        }
    }
}

impl<T: Clone> Default for Event<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Event backed by Lua callbacks for communicating between scripts.
pub struct LuaEvent {
    callbacks: Vec<Function>,
}

impl LuaEvent {
    /// Create a new empty Lua event.
    pub fn new() -> Self {
        Self { callbacks: Vec::new() }
    }

    /// Emit a value to all subscribed callbacks.
    pub fn emit(&mut self, val: Value) {
        for f in &self.callbacks {
            let _ = f.call::<()>(val.clone());
        }
    }

    /// Emit a string to all callbacks.
    pub fn emit_string(&mut self, text: &str) {
        for f in &self.callbacks {
            let _ = f.call::<()>(text);
        }
    }

    /// Add a new callback to this event.
    pub fn subscribe(&mut self, func: Function) {
        self.callbacks.push(func);
    }
}

impl UserData for LuaEvent {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("emit", |_, this, val: Value| {
            this.emit(val);
            Ok(())
        });
        methods.add_method_mut("subscribe", |_, this, func: Function| {
            this.subscribe(func);
            Ok(())
        });
    }
}

use crate::ecs::Entity;
use crate::engine::engine::Engine;
use mlua::Value as LuaValue;
use std::collections::HashMap;

/// Stores events for a single scene so names don't clash across scenes.
pub struct SceneEvents {
    /// Per-entity Lua events keyed by `(Entity, name)`.
    pub script_events: HashMap<(Entity, String), LuaEvent>,
    /// Global events accessible to any script in the scene.
    pub global_events: HashMap<String, Vec<Box<dyn FnMut(&mut Engine, Entity, LuaValue)>>>,
}

impl SceneEvents {
    /// Create an empty scene event container.
    pub fn new() -> Self {
        Self { script_events: HashMap::new(), global_events: HashMap::new() }
    }
}

impl Default for SceneEvents {
    fn default() -> Self {
        Self::new()
    }
}
