//! Lua language tooling used by Vetrace Studio and reusable native tools.

mod completion;
mod constants;
mod diagnostics;
mod formatting;
mod highlighting;
mod service;
mod symbols;

pub use service::LuaLanguageService;

pub(crate) use completion::lua_completions;
pub(crate) use constants::{BUILTINS, KEYWORDS};
pub(crate) use diagnostics::diagnostic_from_error;
pub(crate) use formatting::format_lua;
pub(crate) use highlighting::tokenize;
pub(crate) use symbols::{
    lua_definition, lua_references, lua_rename_edits, lua_signature_help, lua_symbols,
    mask_lua_literals,
};

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use vetrace_script_editor::{
        CompletionComponent, CompletionContext, HighlightKind, LanguageContext,
        ScriptLanguageService,
    };

    #[test]
    fn highlights_keywords_strings_and_comments() {
        let spans = LuaLanguageService.highlight("local x = \"value\" -- comment");
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Keyword));
        assert!(spans.iter().any(|span| span.kind == HighlightKind::String));
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Comment));
    }

    #[test]
    fn syntax_errors_have_a_line_and_quick_fix_for_missing_end() {
        let diagnostics = LuaLanguageService.diagnose(Path::new("test.lua"), "if true then\nprint('x')\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].position.line >= 1);
        assert!(!diagnostics[0].actions.is_empty());
    }

    #[test]
    fn reflected_components_drive_completions() {
        let language = LanguageContext {
            components: vec![CompletionComponent {
                stable_id: "game.health".into(),
                display_name: "Health".into(),
                aliases: vec!["Health".into()],
                fields: vec!["current".into(), "maximum".into()],
            }],
            input_actions: Vec::new(),
        };
        let source = "self.components.Health.c";
        let items = LuaLanguageService.completions(CompletionContext {
            source,
            cursor_byte: source.len(),
            language: &language,
        });
        assert_eq!(items[0].label, "current");
    }

    #[test]
    fn finds_definitions_references_and_signatures() {
        let source = "local speed = 3\nlocal function move(self, dt)\n  return speed * dt\nend\nmove(self, 1)";
        let usage = source.rfind("speed").unwrap();
        let definition = LuaLanguageService.definition(source, usage).unwrap();
        assert_eq!(&source[definition.start..definition.end], "speed");
        assert_eq!(LuaLanguageService.references(source, usage).len(), 2);
        let call = source.len() - 1;
        let help = LuaLanguageService.signature_help(source, call).unwrap();
        assert!(help.label.starts_with("move("));
    }

    #[test]
    fn rename_ignores_strings_and_comments() {
        let source = "local value = 1 -- value\nprint('value', value)";
        let edits = LuaLanguageService.rename_edits(source, source.rfind("value").unwrap(), "score").unwrap();
        assert_eq!(edits.len(), 2);
    }

    #[test]
    fn formatter_is_deterministic() {
        assert_eq!(format_lua("if true then\nprint('x')\nend"), "if true then\n    print('x')\nend\n");
    }

    #[test]
    fn formatter_ignores_keywords_inside_strings_and_comments() {
        let source = "if true then\nprint('end then {') -- end\nprint([[else until}]])\nend";
        assert_eq!(
            format_lua(source),
            "if true then\n    print('end then {') -- end\n    print([[else until}]])\nend\n"
        );
    }
}
