use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPosition {
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode scalar column.
    pub column: usize,
}

impl TextPosition {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    /// UTF-8 byte offset, inclusive.
    pub start: usize,
    /// UTF-8 byte offset, exclusive.
    pub end: usize,
}

impl TextRange {
    pub const fn new(start: usize, end: usize) -> Self { Self { start, end } }
    pub const fn is_empty(self) -> bool { self.start >= self.end }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: TextRange,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeAction {
    pub title: String,
    pub edits: Vec<TextEdit>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub range: TextRange,
    pub position: TextPosition,
    pub code: Option<String>,
    pub actions: Vec<CodeAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HighlightKind {
    Plain,
    Keyword,
    Builtin,
    Type,
    Function,
    Property,
    Number,
    String,
    Comment,
    Operator,
    Punctuation,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HighlightSpan {
    pub range: TextRange,
    pub kind: HighlightKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionKind {
    Keyword,
    Function,
    Method,
    Property,
    Component,
    InputAction,
    Snippet,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub insert_text: String,
    pub detail: String,
    pub kind: CompletionKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompletionComponent {
    pub stable_id: String,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub fields: Vec<String>,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Local,
    Parameter,
    Property,
    Module,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: TextRange,
    pub selection_range: TextRange,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureHelp {
    pub label: String,
    pub parameters: Vec<String>,
    pub active_parameter: usize,
    pub documentation: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExternalChangeKind {
    Modified,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalChange {
    pub document_index: usize,
    pub path: PathBuf,
    pub kind: ExternalChangeKind,
    pub disk_text: Option<String>,
    pub has_local_changes: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalChangeResolution {
    Reload,
    KeepLocal,
    Merge,
}

#[derive(Clone, Debug, Default)]
pub struct LanguageContext {
    pub components: Vec<CompletionComponent>,
    pub input_actions: Vec<String>,
}

pub struct CompletionContext<'a> {
    pub source: &'a str,
    pub cursor_byte: usize,
    pub language: &'a LanguageContext,
}
