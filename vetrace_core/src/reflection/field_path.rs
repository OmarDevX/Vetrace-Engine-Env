use std::fmt;

use super::ReflectionError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldSegment {
    Field(String),
    /// Zero-based array index. Lua bindings translate Lua's one-based indexes.
    Index(usize),
}

/// Parsed path into a reflected component value.
///
/// Supported syntax includes `translation.x`, `items[0].amount`, and root (`""`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FieldPath {
    segments: Vec<FieldSegment>,
}

impl FieldPath {
    pub fn root() -> Self { Self::default() }
    pub fn new(segments: Vec<FieldSegment>) -> Self { Self { segments } }
    pub fn segments(&self) -> &[FieldSegment] { &self.segments }
    pub fn is_root(&self) -> bool { self.segments.is_empty() }

    pub fn field(mut self, name: impl Into<String>) -> Self {
        self.segments.push(FieldSegment::Field(name.into()));
        self
    }

    pub fn index(mut self, index: usize) -> Self {
        self.segments.push(FieldSegment::Index(index));
        self
    }

    pub fn push_field(&mut self, name: impl Into<String>) { self.segments.push(FieldSegment::Field(name.into())); }
    pub fn push_index(&mut self, index: usize) { self.segments.push(FieldSegment::Index(index)); }

    pub fn parse(path: &str) -> Result<Self, ReflectionError> {
        let bytes = path.as_bytes();
        let mut segments = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'.' {
                index += 1;
                continue;
            }
            if bytes[index] == b'[' {
                index += 1;
                let start = index;
                while index < bytes.len() && bytes[index].is_ascii_digit() { index += 1; }
                if start == index || index >= bytes.len() || bytes[index] != b']' {
                    return Err(ReflectionError::InvalidPath(path.to_owned()));
                }
                let parsed = path[start..index]
                    .parse::<usize>()
                    .map_err(|_| ReflectionError::InvalidPath(path.to_owned()))?;
                segments.push(FieldSegment::Index(parsed));
                index += 1;
                continue;
            }
            let start = index;
            while index < bytes.len() && bytes[index] != b'.' && bytes[index] != b'[' { index += 1; }
            if start == index {
                return Err(ReflectionError::InvalidPath(path.to_owned()));
            }
            segments.push(FieldSegment::Field(path[start..index].to_owned()));
        }
        Ok(Self { segments })
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for segment in &self.segments {
            match segment {
                FieldSegment::Field(name) => {
                    if !first { formatter.write_str(".")?; }
                    formatter.write_str(name)?;
                }
                FieldSegment::Index(index) => write!(formatter, "[{index}]")?,
            }
            first = false;
        }
        Ok(())
    }
}

impl std::str::FromStr for FieldPath {
    type Err = ReflectionError;
    fn from_str(value: &str) -> Result<Self, Self::Err> { Self::parse(value) }
}
