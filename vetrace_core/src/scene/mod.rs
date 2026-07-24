use std::collections::HashMap;

use serde_json::Value;

/// Runtime-neutral scene description.
///
/// Plugin-specific components are stored as JSON blobs keyed by component name.
/// Runtime/app layers decide which plugin handles each component.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SceneDef {
    pub entities: Vec<EntityDef>,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EntityDef {
    #[serde(default)]
    pub id: Option<crate::ActorId>,
    pub name: Option<String>,
    pub components: HashMap<String, Value>,
}
