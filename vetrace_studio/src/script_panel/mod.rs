use std::path::PathBuf;
use std::time::{Duration, Instant};

use vetrace_render::egui;
use vetrace_scripting_lua::{LuaDebugValue, LuaDebuggerCommand};
use vetrace_script_editor::{
    CompletionContext, DiagnosticSeverity, ExternalChangeKind, ExternalChangeResolution,
    HighlightKind, HighlightSpan, LanguageContext, ScriptDiagnostic, SymbolKind, TextRange,
};

use crate::script_workspace::{ScriptViewState, StudioScripts, StudioScriptState};
use crate::protocol::{StudioCommand, StudioSnapshot};

mod completion;
mod confirmations;
mod debug_values;
mod debugger;
mod diagnostics;
mod editor;
mod editor_assists;
mod file_actions;
mod highlighting;
mod navigation;
mod search;
mod shell;
mod state_sync;
mod tabs;
mod toolbar;

use debug_values::debug_value_summary;
use editor_assists::{
    apply_editor_assists, project_relative_script, set_text_edit_cursor,
};
use highlighting::{
    byte_column, char_index_to_byte, highlighted_layout_job, line_byte_range, line_for_offset,
};


pub struct ScriptEditorPanel {
    search: String,
    replace: String,
    show_search: bool,
    cursor_byte: usize,
    active_line: usize,
    target_line: Option<usize>,
    completion_open: bool,
    pending_close: Option<usize>,
    pending_cursor_byte: Option<usize>,
    active_path: Option<PathBuf>,
    show_rename_symbol: bool,
    rename_symbol: String,
    show_file_actions: bool,
    rename_file_path: String,
    pending_delete: Option<usize>,
    references: Vec<TextRange>,
    show_outline: bool,
    last_edit: Option<Instant>,
    status: Option<String>,
    watches_text: String,
    watches_initialized: bool,
}

impl Default for ScriptEditorPanel {
    fn default() -> Self {
        Self {
            search: String::new(),
            replace: String::new(),
            show_search: false,
            cursor_byte: 0,
            active_line: 1,
            target_line: None,
            completion_open: false,
            pending_close: None,
            pending_cursor_byte: None,
            active_path: None,
            show_rename_symbol: false,
            rename_symbol: String::new(),
            show_file_actions: false,
            rename_file_path: String::new(),
            pending_delete: None,
            references: Vec::new(),
            show_outline: false,
            last_edit: None,
            status: None,
            watches_text: String::new(),
            watches_initialized: false,
        }
    }
}
