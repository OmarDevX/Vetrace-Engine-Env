use std::any::TypeId;
use std::collections::BTreeMap;

use serde::{de::DeserializeOwned, Serialize};

use crate::reflection::{
    merge_dynamic_patch, ComponentSchema, DynamicValue, FieldKind, FieldPath, FieldSchema,
    FieldSegment, VetraceComponent, VetraceEnum,
};
use crate::{Actor, Component, Engine};

pub type SerializeComponentFn = fn(&Engine, Actor) -> Option<serde_json::Value>;
pub type DeserializeComponentFn = fn(&mut Engine, Actor, serde_json::Value) -> Result<(), String>;
pub type CloneComponentFn = fn(&mut Engine, Actor, Actor) -> Result<(), String>;
pub type CreateComponentFn = fn(&mut Engine, Actor, Option<DynamicValue>) -> Result<(), String>;
pub type RemoveComponentFn = fn(&mut Engine, Actor) -> bool;
pub type InspectorFn = fn(&mut Engine, Actor) -> bool;

mod descriptor;
mod helpers;
mod manager;

pub use descriptor::ComponentDescriptor;
pub use manager::ComponentManager;

use helpers::*;

#[cfg(test)]
mod reflected_marker_tests {
    use super::*;

    #[derive(Default, serde::Deserialize, serde::Serialize)]
    struct UnitMarker;

    #[derive(Default, serde::Deserialize, serde::Serialize)]
    struct EmptyObjectComponent {}

    #[test]
    fn reflected_unit_marker_accepts_an_empty_object() {
        let default = DynamicValue::from_serialize(&UnitMarker).unwrap();
        let value = DynamicValue::Object(std::collections::BTreeMap::new());
        assert!(reflected_component_from_value::<UnitMarker>(default, value).is_ok());
    }

    #[test]
    fn reflected_empty_object_component_remains_an_object() {
        let default = DynamicValue::from_serialize(&EmptyObjectComponent::default()).unwrap();
        let value = DynamicValue::Object(std::collections::BTreeMap::new());
        assert!(reflected_component_from_value::<EmptyObjectComponent>(default, value).is_ok());
    }
}
