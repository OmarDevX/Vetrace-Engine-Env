//! Minimal RPC dispatch placeholder.

use serde_json::Value;
use std::collections::HashMap;

use crate::ecs::Entity;

/// Function signature for RPC handlers.
pub type RpcHandler = fn(Entity, Vec<Value>);

/// Table mapping method names to handler functions.
#[derive(Default)]
pub struct RpcTable {
    handlers: HashMap<String, RpcHandler>,
}

impl RpcTable {
    /// Register a new RPC handler.
    pub fn register(&mut self, name: &str, handler: RpcHandler) {
        self.handlers.insert(name.to_string(), handler);
    }

    /// Dispatch a RPC if the method exists.
    pub fn dispatch(&self, entity: Entity, method: &str, args: Vec<Value>) {
        if let Some(&h) = self.handlers.get(method) {
            h(entity, args);
        }
    }
}
