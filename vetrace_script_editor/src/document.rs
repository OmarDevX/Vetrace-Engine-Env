use std::path::PathBuf;

use crate::{HighlightSpan, LanguageError, ScriptDiagnostic, ScriptLanguageService, TextEdit};

#[derive(Clone, Debug)]
pub struct ScriptDocument {
    pub path: PathBuf,
    pub language_id: String,
    pub text: String,
    pub(crate) saved_text: String,
    pub revision: u64,
    pub diagnostics_revision: u64,
    pub diagnostics: Vec<ScriptDiagnostic>,
    pub highlights: Vec<HighlightSpan>,
}

impl ScriptDocument {
    pub fn from_text(
        path: impl Into<PathBuf>,
        language_id: impl Into<String>,
        text: String,
    ) -> Self {
        Self {
            path: path.into(),
            language_id: language_id.into(),
            saved_text: text.clone(),
            text,
            revision: 0,
            diagnostics_revision: u64::MAX,
            diagnostics: Vec::new(),
            highlights: Vec::new(),
        }
    }

    pub fn is_dirty(&self) -> bool { self.text != self.saved_text }
    pub fn saved_text(&self) -> &str { &self.saved_text }

    pub fn set_path(&mut self, path: impl Into<PathBuf>) {
        self.path = path.into();
        self.revision = self.revision.saturating_add(1);
    }

    pub fn set_text(&mut self, text: String) {
        if self.text != text {
            self.text = text;
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub fn mark_saved(&mut self) { self.saved_text = self.text.clone(); }

    pub fn replace_from_disk(&mut self, text: String) {
        self.text = text.clone();
        self.saved_text = text;
        self.revision = self.revision.saturating_add(1);
        self.diagnostics_revision = u64::MAX;
    }

    pub fn apply_edits(&mut self, edits: &[TextEdit]) -> Result<(), LanguageError> {
        let mut edits = edits.to_vec();
        edits.sort_by(|left, right| right.range.start.cmp(&left.range.start));
        let mut source = self.text.clone();
        for edit in edits {
            if edit.range.start > edit.range.end || edit.range.end > source.len() {
                return Err(LanguageError("quick fix contains an invalid text range".into()));
            }
            if !source.is_char_boundary(edit.range.start) || !source.is_char_boundary(edit.range.end) {
                return Err(LanguageError("quick fix range is not on UTF-8 character boundaries".into()));
            }
            source.replace_range(edit.range.start..edit.range.end, &edit.replacement);
        }
        self.set_text(source);
        Ok(())
    }

    pub fn update_analysis(&mut self, service: &dyn ScriptLanguageService) {
        self.highlights = service.highlight(&self.text);
        self.diagnostics = service.diagnose(&self.path, &self.text);
        self.diagnostics_revision = self.revision;
    }
}
