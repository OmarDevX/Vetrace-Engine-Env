//! Editor-independent script document and language-service abstractions.
//!
//! This crate deliberately has no egui, Lua, renderer, runtime, or filesystem
//! watcher dependency. Studio and native Rust tools can provide their own UI
//! while sharing document, diagnostic, completion, and quick-fix behavior.

mod document;
mod language;
mod merge;
mod text;
mod types;
mod workspace;

pub use document::ScriptDocument;
pub use language::{LanguageError, LanguageRegistry, ScriptLanguageService};
pub use merge::three_way_merge;
pub use text::{line_range, offset_to_position};
pub use types::*;
pub use workspace::ScriptWorkspace;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    struct Plain;
    impl ScriptLanguageService for Plain {
        fn language_id(&self) -> &'static str { "plain" }
        fn display_name(&self) -> &'static str { "Plain" }
        fn extensions(&self) -> &'static [&'static str] { &["txt"] }
        fn highlight(&self, _source: &str) -> Vec<HighlightSpan> { Vec::new() }
        fn diagnose(&self, _path: &Path, _source: &str) -> Vec<ScriptDiagnostic> { Vec::new() }
        fn completions(&self, _context: CompletionContext<'_>) -> Vec<CompletionItem> { Vec::new() }
    }

    #[test]
    fn edits_apply_from_the_end_without_shifting_ranges() {
        let mut document = ScriptDocument::from_text("test.txt", "plain", "abc def".into());
        document.apply_edits(&[
            TextEdit { range: TextRange::new(0, 3), replacement: "ABC".into() },
            TextEdit { range: TextRange::new(4, 7), replacement: "DEF".into() },
        ]).unwrap();
        assert_eq!(document.text, "ABC DEF");
    }

    #[test]
    fn registry_replaces_extension_ownership() {
        let mut registry = LanguageRegistry::new();
        registry.register(Plain);
        assert_eq!(registry.for_path(Path::new("thing.txt")).unwrap().language_id(), "plain");
    }

    #[test]
    fn positions_are_one_based() {
        assert_eq!(offset_to_position("one\ntwo", 5), TextPosition::new(2, 2));
        assert_eq!(line_range("one\ntwo", 2), TextRange::new(4, 7));
    }

    #[test]
    fn three_way_merge_preserves_independent_edits() {
        let base = "first\nsecond\nthird\n";
        let local = "FIRST\nsecond\nthird\n";
        let disk = "first\nsecond\nTHIRD\n";
        assert_eq!(three_way_merge(base, local, disk), "FIRST\nsecond\nTHIRD\n");
    }

    #[test]
    fn three_way_merge_marks_conflicting_lines() {
        let merged = three_way_merge("same\n", "local\n", "disk\n");
        assert!(merged.contains("<<<<<<< Studio"));
        assert!(merged.contains("local"));
        assert!(merged.contains("disk"));
    }

    #[test]
    fn workspace_detects_and_resolves_external_changes() {
        let root = std::env::temp_dir().join(format!(
            "vetrace-script-workspace-{}",
            std::process::id(),
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.txt");
        std::fs::write(&path, "saved\n").unwrap();
        let mut registry = LanguageRegistry::new();
        registry.register(Plain);
        let mut workspace = ScriptWorkspace::new(registry);
        let index = workspace.open(&path).unwrap();
        workspace.documents_mut()[index].set_text("local\n".to_owned());
        std::fs::write(&path, "disk\n").unwrap();
        let change = workspace.poll_external_changes().into_iter().next().unwrap();
        assert!(change.has_local_changes);
        workspace.resolve_external_change(&change, ExternalChangeResolution::Merge).unwrap();
        assert!(workspace.documents()[index].text.contains("<<<<<<< Studio"));
        let _ = std::fs::remove_dir_all(root);
    }
}
