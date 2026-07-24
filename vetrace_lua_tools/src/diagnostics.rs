use vetrace_script_editor::{
    line_range, CodeAction, DiagnosticSeverity, ScriptDiagnostic, TextEdit, TextPosition, TextRange,
};

pub(crate) fn diagnostic_from_error(source: &str, message: &str) -> ScriptDiagnostic {
    let (line, column, clean_message) = parse_lua_error(message);
    let range = diagnostic_range(source, line, column);
    let mut actions = Vec::new();
    let lower = clean_message.to_ascii_lowercase();
    if lower.contains("expected 'end'") || lower.contains("'end' expected") || lower.contains("near <eof>") {
        let prefix = if source.ends_with('\n') || source.is_empty() { "" } else { "\n" };
        actions.push(CodeAction {
            title: "Insert missing `end`".into(),
            edits: vec![TextEdit {
                range: TextRange::new(source.len(), source.len()),
                replacement: format!("{prefix}end\n"),
            }],
        });
    }
    ScriptDiagnostic {
        severity: DiagnosticSeverity::Error,
        message: clean_message,
        range,
        position: TextPosition::new(line, column),
        code: Some("lua.syntax".into()),
        actions,
    }
}

fn parse_lua_error(message: &str) -> (usize, usize, String) {
    // Lua errors usually end in `:<line>: <message>`. Some builds include a
    // column after the line, so accept both forms without depending on the
    // exact mlua/Lua wording.
    let mut line = 1usize;
    let mut column = 1usize;
    let mut clean = message.trim().to_owned();
    let pieces = message.split(':').collect::<Vec<_>>();
    for index in 0..pieces.len() {
        let Ok(candidate_line) = pieces[index].trim().parse::<usize>() else { continue; };
        line = candidate_line.max(1);
        let mut message_index = index + 1;
        if let Some(next) = pieces.get(index + 1).and_then(|piece| piece.trim().parse::<usize>().ok()) {
            column = next.max(1);
            message_index += 1;
        }
        clean = pieces[message_index..].join(":").trim().to_owned();
        break;
    }
    if clean.is_empty() { clean = message.trim().to_owned(); }
    (line, column, clean)
}

fn diagnostic_range(source: &str, line: usize, column: usize) -> TextRange {
    let line = line_range(source, line);
    if line.is_empty() { return line; }
    let mut start = line.start;
    let mut seen = 1usize;
    for (offset, character) in source[line.start..line.end].char_indices() {
        if seen >= column {
            start = line.start + offset;
            break;
        }
        seen += 1;
        start = line.start + offset + character.len_utf8();
    }
    let end = source[start..line.end]
        .char_indices()
        .find_map(|(offset, character)| (!character.is_alphanumeric() && character != '_').then_some(start + offset))
        .unwrap_or(line.end)
        .max((start + source[start..].chars().next().map(char::len_utf8).unwrap_or(0)).min(source.len()));
    TextRange::new(start, end)
}
