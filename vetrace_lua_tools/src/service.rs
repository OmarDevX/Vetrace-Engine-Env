use std::path::Path;

use mlua::Lua;
use vetrace_script_editor::{
    CompletionContext, CompletionItem, HighlightSpan, LanguageError, ScriptDiagnostic,
    ScriptLanguageService, ScriptSymbol, SignatureHelp, TextEdit, TextRange,
};

use crate::{
    diagnostic_from_error, format_lua, lua_completions, lua_definition, lua_references,
    lua_rename_edits, lua_signature_help, lua_symbols, tokenize,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct LuaLanguageService;

impl ScriptLanguageService for LuaLanguageService {
    fn language_id(&self) -> &'static str { "lua" }
    fn display_name(&self) -> &'static str { "Lua" }
    fn extensions(&self) -> &'static [&'static str] { &["lua"] }

    fn highlight(&self, source: &str) -> Vec<HighlightSpan> {
        tokenize(source)
    }

    fn diagnose(&self, path: &Path, source: &str) -> Vec<ScriptDiagnostic> {
        let lua = Lua::new();
        match lua.load(source).set_name(path.to_string_lossy().as_ref()).into_function() {
            Ok(_) => Vec::new(),
            Err(error) => vec![diagnostic_from_error(source, &error.to_string())],
        }
    }

    fn completions(&self, context: CompletionContext<'_>) -> Vec<CompletionItem> {
        lua_completions(context)
    }

    fn symbols(&self, source: &str) -> Vec<ScriptSymbol> {
        lua_symbols(source)
    }

    fn definition(&self, source: &str, cursor_byte: usize) -> Option<TextRange> {
        lua_definition(source, cursor_byte)
    }

    fn references(&self, source: &str, cursor_byte: usize) -> Vec<TextRange> {
        lua_references(source, cursor_byte)
    }

    fn rename_edits(
        &self,
        source: &str,
        cursor_byte: usize,
        new_name: &str,
    ) -> Result<Vec<TextEdit>, LanguageError> {
        lua_rename_edits(source, cursor_byte, new_name)
    }

    fn signature_help(&self, source: &str, cursor_byte: usize) -> Option<SignatureHelp> {
        lua_signature_help(source, cursor_byte)
    }

    fn format(&self, source: &str) -> Result<String, LanguageError> {
        Ok(format_lua(source))
    }
}
