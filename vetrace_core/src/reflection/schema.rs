use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::Component;

use super::{humanize_identifier, DynamicValue};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Null,
    Boolean,
    Integer,
    UnsignedInteger,
    Number,
    String,
    Vec2,
    Vec3,
    Vec4,
    Quaternion,
    Color,
    Enum,
    AssetPath,
    EntityReference,
    Array,
    Object,
    Unknown,
}

impl FieldKind {
    pub fn infer(value: &DynamicValue) -> Self {
        match value {
            DynamicValue::Null => Self::Null,
            DynamicValue::Bool(_) => Self::Boolean,
            DynamicValue::I64(_) => Self::Integer,
            DynamicValue::U64(_) => Self::UnsignedInteger,
            DynamicValue::F64(_) => Self::Number,
            DynamicValue::String(_) => Self::String,
            DynamicValue::Array(values) => match values.len() {
                2 if values.iter().all(is_number) => Self::Vec2,
                3 if values.iter().all(is_number) => Self::Vec3,
                4 if values.iter().all(is_number) => Self::Vec4,
                _ => Self::Array,
            },
            DynamicValue::Object(_) => Self::Object,
        }
    }
}

fn is_number(value: &DynamicValue) -> bool {
    matches!(value, DynamicValue::I64(_) | DynamicValue::U64(_) | DynamicValue::F64(_))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumericRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}

impl NumericRange {
    pub const fn new(min: Option<f64>, max: Option<f64>, step: Option<f64>) -> Self {
        Self { min, max, step }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub display_name: String,
    pub kind: FieldKind,
    pub default_value: DynamicValue,
    /// Serialized enum variant names accepted by serde. Empty for non-enum
    /// fields or enums whose owning plugin did not publish metadata.
    #[serde(default)]
    pub enum_variants: Vec<String>,
    #[serde(default = "default_true")]
    pub editable: bool,
    #[serde(default = "default_true")]
    pub serializable: bool,
    #[serde(default = "default_true")]
    pub lua_accessible: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub numeric_range: Option<NumericRange>,
    #[serde(default)]
    pub children: Vec<FieldSchema>,
}

impl FieldSchema {
    pub fn inferred<T: Serialize + ?Sized>(name: impl Into<String>, value: &T) -> Self {
        let name = name.into();
        let default_value = DynamicValue::from_serialize(value).unwrap_or_default();
        Self::from_dynamic(name.clone(), humanize_identifier(&name), default_value)
    }

    pub fn from_dynamic(name: impl Into<String>, display_name: impl Into<String>, value: DynamicValue) -> Self {
        let name = name.into();
        let children = infer_children(&value);
        Self {
            display_name: display_name.into(),
            name,
            kind: FieldKind::infer(&value),
            default_value: value,
            enum_variants: Vec::new(),
            editable: true,
            serializable: true,
            lua_accessible: true,
            description: None,
            numeric_range: None,
            children,
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    pub fn with_kind(mut self, kind: FieldKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_enum_variants<I, S>(mut self, variants: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.kind = FieldKind::Enum;
        self.enum_variants = variants.into_iter().map(Into::into).collect();
        self
    }

    pub fn read_only(mut self) -> Self {
        self.editable = false;
        self
    }

    pub fn hidden_from_lua(mut self) -> Self {
        self.lua_accessible = false;
        self
    }

    pub fn runtime_only(mut self) -> Self {
        self.serializable = false;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_range(mut self, min: Option<f64>, max: Option<f64>, step: Option<f64>) -> Self {
        self.numeric_range = Some(NumericRange::new(min, max, step));
        self
    }
}

fn infer_children(value: &DynamicValue) -> Vec<FieldSchema> {
    match value {
        DynamicValue::Object(values) => values
            .iter()
            .map(|(name, value)| {
                FieldSchema::from_dynamic(name.clone(), humanize_identifier(name), value.clone())
            })
            .collect(),
        DynamicValue::Array(values) if !values.is_empty() => vec![FieldSchema::from_dynamic(
            "item",
            "Item",
            values[0].clone(),
        )],
        _ => Vec::new(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentSchema {
    pub stable_id: String,
    pub display_name: String,
    pub category: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_true")]
    pub constructible: bool,
    #[serde(default = "default_true")]
    pub removable: bool,
    #[serde(default = "default_true")]
    pub serializable: bool,
    #[serde(default = "default_true")]
    pub lua_accessible: bool,
    #[serde(default)]
    pub fields: Vec<FieldSchema>,
}

impl ComponentSchema {
    pub fn new(
        stable_id: impl Into<String>,
        display_name: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            stable_id: stable_id.into(),
            display_name: display_name.into(),
            category: category.into(),
            description: None,
            constructible: true,
            removable: true,
            serializable: true,
            lua_accessible: true,
            fields: Vec::new(),
        }
    }

    pub fn inferred(
        stable_id: impl Into<String>,
        display_name: impl Into<String>,
        category: impl Into<String>,
        value: DynamicValue,
    ) -> Self {
        let mut schema = Self::new(stable_id, display_name, category);
        schema.fields = match value {
            DynamicValue::Object(values) => values
                .into_iter()
                .map(|(name, value)| {
                    FieldSchema::from_dynamic(name.clone(), humanize_identifier(&name), value)
                })
                .collect(),
            other => vec![FieldSchema::from_dynamic("value", "Value", other)],
        };
        schema
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

fn default_true() -> bool { true }

/// Metadata contract for unit enums exposed through generic reflection.
///
/// Derive `VetraceEnum` on an enum, then register it for any reflected field
/// using `ComponentManager::register_enum_field::<MyEnum>(...)`. Custom
/// components using `VetraceComponent` can instead mark a field with
/// `#[vetrace(enum_options)]`.
pub trait VetraceEnum: 'static {
    fn variants() -> &'static [&'static str];
}

/// Opt-in metadata contract for a fully reflected component.
///
/// Components do not need to implement this trait merely to be generically
/// readable/writable: `ComponentManager::register_serializable` already exposes
/// their serde representation. Implementing/deriving this trait additionally
/// provides a default constructor and stable editor schema.
pub trait VetraceComponent:
    Component + Clone + Default + Serialize + DeserializeOwned
{
    const STABLE_ID: &'static str;
    const DISPLAY_NAME: &'static str;
    const CATEGORY: &'static str;

    fn component_schema() -> ComponentSchema;
}
