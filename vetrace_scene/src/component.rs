use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Generic typed scene component payload.
///
/// The document format is deliberately component-open:
///
/// ```json
/// { "type": "vetrace.physics.collider", "data": { ... } }
/// ```
///
/// `vetrace_scene` knows how to instantiate built-in component IDs, but it no
/// longer owns a hardcoded enum variant for every subsystem. New crates can add
/// new authored component IDs while old scene files remain parseable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneComponent {
    #[serde(rename = "type", alias = "kind")]
    pub type_id: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

impl SceneComponent {
    pub fn new<T: Serialize>(type_id: impl Into<String>, data: T) -> Self {
        Self {
            type_id: type_id.into(),
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
        }
    }

    pub fn raw(type_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self { type_id: type_id.into(), data }
    }

    pub fn matches(&self, type_id: &str) -> bool { self.type_id == type_id }

    pub fn matches_any(&self, aliases: &[&str]) -> bool {
        aliases.iter().any(|alias| self.type_id == *alias)
    }

    pub fn decode<T: DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_value(self.data.clone()).ok()
    }
}
