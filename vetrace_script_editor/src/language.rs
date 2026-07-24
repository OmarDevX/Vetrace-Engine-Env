use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

use crate::{
    CompletionContext, CompletionItem, HighlightSpan, ScriptDiagnostic, ScriptSymbol,
    SignatureHelp, TextEdit, TextRange,
};

pub trait ScriptLanguageService: Send + Sync + 'static {
    fn language_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn highlight(&self, source: &str) -> Vec<HighlightSpan>;
    fn diagnose(&self, path: &Path, source: &str) -> Vec<ScriptDiagnostic>;
    fn completions(&self, context: CompletionContext<'_>) -> Vec<CompletionItem>;
    fn symbols(&self, _source: &str) -> Vec<ScriptSymbol> { Vec::new() }
    fn definition(&self, _source: &str, _cursor_byte: usize) -> Option<TextRange> { None }
    fn references(&self, _source: &str, _cursor_byte: usize) -> Vec<TextRange> { Vec::new() }
    fn rename_edits(
        &self,
        _source: &str,
        _cursor_byte: usize,
        _new_name: &str,
    ) -> Result<Vec<TextEdit>, LanguageError> {
        Err(LanguageError("rename is not supported by this language service".into()))
    }
    fn signature_help(&self, _source: &str, _cursor_byte: usize) -> Option<SignatureHelp> { None }
    fn format(&self, source: &str) -> Result<String, LanguageError> {
        Ok(source.to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageError(pub String);

impl fmt::Display for LanguageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LanguageError {}

#[derive(Default)]
pub struct LanguageRegistry {
    services: BTreeMap<String, Arc<dyn ScriptLanguageService>>,
    extensions: BTreeMap<String, String>,
}

impl LanguageRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register<S: ScriptLanguageService>(&mut self, service: S) {
        let service: Arc<dyn ScriptLanguageService> = Arc::new(service);
        let id = service.language_id().to_owned();
        self.extensions.retain(|_, owner| owner != &id);
        for extension in service.extensions() {
            self.extensions.insert(extension.to_ascii_lowercase(), id.clone());
        }
        self.services.insert(id, service);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn ScriptLanguageService>> {
        self.services.get(id).cloned()
    }

    pub fn for_path(&self, path: &Path) -> Option<Arc<dyn ScriptLanguageService>> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        let id = self.extensions.get(&extension)?;
        self.get(id)
    }
}
