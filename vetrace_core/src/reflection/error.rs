use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReflectionError {
    ComponentNotRegistered(String),
    ComponentNotPresent(String),
    ComponentNotReadable(String),
    ComponentReadOnly(String),
    ComponentNotConstructible(String),
    ComponentNotRemovable(String),
    LuaAccessDenied(String),
    InvalidPath(String),
    MissingField(String),
    IndexOutOfBounds { path: String, index: usize, length: usize },
    TypeMismatch { path: String, expected: &'static str, actual: &'static str },
    Serialization(String),
    Deserialization(String),
    Operation(String),
}

impl fmt::Display for ReflectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComponentNotRegistered(id) => write!(formatter, "component `{id}` is not registered"),
            Self::ComponentNotPresent(id) => write!(formatter, "entity does not contain component `{id}`"),
            Self::ComponentNotReadable(id) => write!(formatter, "component `{id}` is not reflectively readable"),
            Self::ComponentReadOnly(id) => write!(formatter, "component `{id}` is read-only"),
            Self::ComponentNotConstructible(id) => write!(formatter, "component `{id}` cannot be constructed generically"),
            Self::ComponentNotRemovable(id) => write!(formatter, "component `{id}` cannot be removed"),
            Self::LuaAccessDenied(id) => write!(formatter, "component `{id}` is not exposed to Lua"),
            Self::InvalidPath(path) => write!(formatter, "invalid field path `{path}`"),
            Self::MissingField(path) => write!(formatter, "field `{path}` does not exist"),
            Self::IndexOutOfBounds { path, index, length } => write!(formatter, "index {index} is outside `{path}` (length {length})"),
            Self::TypeMismatch { path, expected, actual } => write!(formatter, "field `{path}` expected {expected}, found {actual}"),
            Self::Serialization(error) => write!(formatter, "reflection serialization failed: {error}"),
            Self::Deserialization(error) => write!(formatter, "reflection deserialization failed: {error}"),
            Self::Operation(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for ReflectionError {}
