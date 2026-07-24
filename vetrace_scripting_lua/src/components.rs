use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

fn default_enabled() -> bool { true }

/// Serializable value exposed by a Lua script property.
///
/// The deliberately small set keeps scene data portable and editor-friendly.
/// More complex engine values should be represented by dedicated component
/// fields or asset/entity references rather than arbitrary Lua tables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScriptValue {
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Null,
}

impl Default for ScriptValue {
    fn default() -> Self { Self::Null }
}

impl ScriptValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Vec2(_) => "vec2",
            Self::Vec3(_) => "vec3",
            Self::Vec4(_) => "vec4",
            Self::Null => "nil",
        }
    }
}

/// Attaches a Lua gameplay script to an entity.
///
/// Authored scenes store a project-relative script path and optional property
/// overrides. During runtime startup the project runtime resolves the path and
/// uses it as the live script key. Runtime lifecycle state is deliberately
/// excluded from scene serialization.
#[derive(Clone, Debug, Serialize, Deserialize, vetrace_core::VetraceComponent)]
#[vetrace_component(
    id = "vetrace.scripting.lua_script",
    display_name = "Lua Script",
    category = "Scripting",
    description = "Attaches a project Lua script to this entity."
)]
pub struct ScriptComponent {
    #[vetrace(
        kind = "asset_path",
        display_name = "Script",
        description = "Lua script under assets/scripts/. Create, browse, type, or drag a script asset here."
    )]
    pub script: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub properties: BTreeMap<String, ScriptValue>,
}

impl Default for ScriptComponent {
    fn default() -> Self {
        Self {
            script: String::new(),
            enabled: true,
            properties: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vetrace_core::{FieldKind, VetraceComponent};

    #[test]
    fn script_path_is_published_as_an_asset_field() {
        let schema = ScriptComponent::component_schema();
        let script = schema.fields.iter().find(|field| field.name == "script").unwrap();
        assert_eq!(script.kind, FieldKind::AssetPath);
        assert!(schema.fields.iter().all(|field| field.name != "started"));
    }
}
