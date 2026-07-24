use std::collections::BTreeMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::error::ReflectionError;
use super::field_path::{FieldPath, FieldSegment};


/// Portable runtime value used by component reflection, Lua, scene tooling,
/// inspectors, undo/redo, and future remote tooling.
///
/// This type deliberately mirrors the data model supported by serde/JSON. Rich
/// editor meanings such as colors, vectors, asset paths, and entity references
/// live in [`FieldKind`] metadata rather than being hardcoded into storage.
#[derive(Clone, Debug, PartialEq)]
pub enum DynamicValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Array(Vec<DynamicValue>),
    Object(BTreeMap<String, DynamicValue>),
}

impl Default for DynamicValue {
    fn default() -> Self { Self::Null }
}

impl DynamicValue {
    pub fn from_serialize<T: Serialize + ?Sized>(value: &T) -> Result<Self, ReflectionError> {
        let value = serde_json::to_value(value)
            .map_err(|error| ReflectionError::Serialization(error.to_string()))?;
        Ok(Self::from_json(value))
    }

    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<T, ReflectionError> {
        serde_json::from_value(self.clone().into_json())
            .map_err(|error| ReflectionError::Deserialization(error.to_string()))
    }

    pub fn from_json(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(value),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Self::I64(value)
                } else if let Some(value) = value.as_u64() {
                    Self::U64(value)
                } else {
                    Self::F64(value.as_f64().unwrap_or_default())
                }
            }
            serde_json::Value::String(value) => Self::String(value),
            serde_json::Value::Array(values) => {
                Self::Array(values.into_iter().map(Self::from_json).collect())
            }
            serde_json::Value::Object(values) => Self::Object(
                values
                    .into_iter()
                    .map(|(name, value)| (name, Self::from_json(value)))
                    .collect(),
            ),
        }
    }

    pub fn into_json(self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(value),
            Self::I64(value) => serde_json::Value::Number(value.into()),
            Self::U64(value) => serde_json::Value::Number(value.into()),
            Self::F64(value) => serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Self::String(value) => serde_json::Value::String(value),
            Self::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(Self::into_json).collect())
            }
            Self::Object(values) => serde_json::Value::Object(
                values
                    .into_iter()
                    .map(|(name, value)| (name, value.into_json()))
                    .collect(),
            ),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::I64(_) => "integer",
            Self::U64(_) => "unsigned integer",
            Self::F64(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
        }
    }

    pub fn get(&self, path: &FieldPath) -> Result<&DynamicValue, ReflectionError> {
        let mut current = self;
        for segment in path.segments() {
            current = match (current, segment) {
                (Self::Object(values), FieldSegment::Field(name)) => values
                    .get(name)
                    .ok_or_else(|| ReflectionError::MissingField(path.to_string())),
                (Self::Array(values), FieldSegment::Index(index)) => values
                    .get(*index)
                    .ok_or_else(|| ReflectionError::IndexOutOfBounds {
                        path: path.to_string(),
                        index: *index,
                        length: values.len(),
                    }),
                (value, _) => Err(ReflectionError::TypeMismatch {
                    path: path.to_string(),
                    expected: match segment {
                        FieldSegment::Field(_) => "object",
                        FieldSegment::Index(_) => "array",
                    },
                    actual: value.type_name(),
                }),
            }?;
        }
        Ok(current)
    }

    pub fn get_mut(&mut self, path: &FieldPath) -> Result<&mut DynamicValue, ReflectionError> {
        let mut current = self;
        for segment in path.segments() {
            current = match segment {
                FieldSegment::Field(name) => {
                    let actual = current.type_name();
                    let Self::Object(values) = current else {
                        return Err(ReflectionError::TypeMismatch {
                            path: path.to_string(),
                            expected: "object",
                            actual,
                        });
                    };
                    values
                        .get_mut(name)
                        .ok_or_else(|| ReflectionError::MissingField(path.to_string()))?
                }
                FieldSegment::Index(index) => {
                    let actual = current.type_name();
                    let Self::Array(values) = current else {
                        return Err(ReflectionError::TypeMismatch {
                            path: path.to_string(),
                            expected: "array",
                            actual,
                        });
                    };
                    let length = values.len();
                    values.get_mut(*index).ok_or_else(|| ReflectionError::IndexOutOfBounds {
                        path: path.to_string(),
                        index: *index,
                        length,
                    })?
                }
            };
        }
        Ok(current)
    }

    pub fn set(&mut self, path: &FieldPath, value: DynamicValue) -> Result<(), ReflectionError> {
        if path.is_root() {
            *self = value;
            return Ok(());
        }
        *self.get_mut(path)? = value;
        Ok(())
    }

    pub fn object(&self) -> Option<&BTreeMap<String, DynamicValue>> {
        match self {
            Self::Object(value) => Some(value),
            _ => None,
        }
    }

    pub fn array(&self) -> Option<&[DynamicValue]> {
        match self {
            Self::Array(value) => Some(value),
            _ => None,
        }
    }
}



/// Recursively overlays `patch` onto `target` while preserving unspecified
/// fields. Objects are merged by key; arrays and scalar values are replaced.
/// This is used when constructing reflected components from partial Lua/editor
/// tables on top of their Rust `Default` value.
pub fn merge_dynamic_patch(target: &mut DynamicValue, patch: DynamicValue) {
    match (target, patch) {
        (DynamicValue::Object(target), DynamicValue::Object(patch)) => {
            for (name, value) in patch {
                match target.get_mut(&name) {
                    Some(existing) => merge_dynamic_patch(existing, value),
                    None => {
                        target.insert(name, value);
                    }
                }
            }
        }
        (target, patch) => *target = patch,
    }
}

impl From<serde_json::Value> for DynamicValue {
    fn from(value: serde_json::Value) -> Self { Self::from_json(value) }
}

impl From<DynamicValue> for serde_json::Value {
    fn from(value: DynamicValue) -> Self { value.into_json() }
}

impl Serialize for DynamicValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.clone().into_json().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DynamicValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_json::Value::deserialize(deserializer).map(Self::from_json)
    }
}
