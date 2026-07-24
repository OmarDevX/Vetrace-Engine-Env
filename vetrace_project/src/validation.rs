use std::fmt;

use crate::{
    CURRENT_PROJECT_FORMAT_VERSION, ProjectError, ProjectManifest, ProjectPaths, ProjectResult,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => formatter.write_str("error"),
            Self::Warning => formatter.write_str("warning"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub code: &'static str,
    pub field: Option<String>,
    pub message: String,
}

impl ValidationIssue {
    fn error(code: &'static str, field: Option<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            code,
            field,
            message: message.into(),
        }
    }

    fn warning(code: &'static str, field: Option<String>, message: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            code,
            field,
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(field) = &self.field {
            write!(formatter, "{} [{}] {field}: {}", self.severity, self.code, self.message)
        } else {
            write!(formatter, "{} [{}]: {}", self.severity, self.code, self.message)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidationReport {
    issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|issue| issue.severity == ValidationSeverity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|issue| issue.severity == ValidationSeverity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.errors().count()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings().count()
    }

    pub fn is_valid(&self) -> bool {
        self.error_count() == 0
    }

    pub fn into_result(self) -> ProjectResult<()> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(ProjectError::Validation(self))
        }
    }

    fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                writeln!(formatter)?;
            }
            write!(formatter, "{issue}")?;
        }
        Ok(())
    }
}

mod files;
mod helpers;
mod manifest;

pub use files::validate_project_files;
pub use manifest::validate_manifest;

use helpers::*;
