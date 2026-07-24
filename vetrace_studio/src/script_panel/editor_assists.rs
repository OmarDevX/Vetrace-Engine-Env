use super::*;

pub(super) fn set_text_edit_cursor(
    ui: &egui::Ui,
    output: &mut egui::text_edit::TextEditOutput,
    source: &str,
    byte_offset: usize,
) {
    let character_index = source[..byte_offset.min(source.len())].chars().count();
    output.state.cursor.set_char_range(Some(egui::text::CCursorRange::one(
        egui::text::CCursor::new(character_index),
    )));
    output.state.clone().store(ui.ctx(), output.response.id);
}

pub(super) fn apply_editor_assists(old: &str, mut edited: String, cursor_byte: usize) -> (String, usize) {
    let Some((start, inserted)) = single_insertion(old, &edited) else {
        let cursor = cursor_byte.min(edited.len());
        return (edited, cursor);
    };
    let mut cursor = cursor_byte.min(edited.len());
    if inserted == "\n" {
        let previous_line_start = old[..start].rfind('\n').map(|index| index + 1).unwrap_or(0);
        let previous_line = &old[previous_line_start..start];
        let base_indent = previous_line.chars().take_while(|character| matches!(character, ' ' | '\t')).collect::<String>();
        let trimmed = previous_line.trim_end();
        let opens_block = trimmed.ends_with("then")
            || trimmed.ends_with(" do")
            || trimmed == "do"
            || trimmed.starts_with("function ")
            || trimmed.contains(" = function(")
            || trimmed == "repeat"
            || trimmed.ends_with('{');
        let indent = if opens_block { format!("{base_indent}    ") } else { base_indent };
        edited.insert_str(cursor, &indent);
        cursor += indent.len();
        return (edited, cursor);
    }

    if inserted.len() == 1 {
        let character = inserted.as_bytes()[0] as char;
        if let Some(closing) = matching_closer(character) {
            if should_insert_pair(old, start, character) {
                edited.insert(cursor, closing);
                return (edited, cursor);
            }
        }
        if matches!(character, ')' | ']' | '}' | '\'' | '"')
            && old[start..].starts_with(character)
        {
            edited.replace_range(start..start + character.len_utf8(), "");
            cursor = (start + character.len_utf8()).min(edited.len());
        }
    }
    (edited, cursor)
}

pub(super) fn single_insertion<'a>(old: &str, edited: &'a str) -> Option<(usize, &'a str)> {
    if edited.len() <= old.len() { return None; }
    let mut prefix = 0usize;
    let common = old.len().min(edited.len());
    while prefix < common && old.as_bytes()[prefix] == edited.as_bytes()[prefix] { prefix += 1; }
    while prefix > 0 && (!old.is_char_boundary(prefix) || !edited.is_char_boundary(prefix)) { prefix -= 1; }
    let mut old_suffix = old.len();
    let mut edited_suffix = edited.len();
    while old_suffix > prefix
        && edited_suffix > prefix
        && old.as_bytes()[old_suffix - 1] == edited.as_bytes()[edited_suffix - 1]
    {
        old_suffix -= 1;
        edited_suffix -= 1;
    }
    while edited_suffix < edited.len() && !edited.is_char_boundary(edited_suffix) { edited_suffix += 1; }
    (old_suffix == prefix).then(|| (prefix, &edited[prefix..edited_suffix]))
}

pub(super) fn matching_closer(character: char) -> Option<char> {
    match character {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '\'' => Some('\''),
        '"' => Some('"'),
        _ => None,
    }
}

pub(super) fn should_insert_pair(source: &str, offset: usize, opener: char) -> bool {
    let next = source[offset..].chars().next();
    if next.is_some_and(|character| character.is_alphanumeric() || character == '_') { return false; }
    if matches!(opener, '\'' | '"') {
        let mut escaped = false;
        let mut inside = false;
        for character in source[..offset].chars() {
            if escaped { escaped = false; continue; }
            if character == '\\' { escaped = true; continue; }
            if character == opener { inside = !inside; }
        }
        !inside
    } else {
        true
    }
}

pub(super) fn project_relative_script(path: &std::path::Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.find("assets/scripts/")
        .map(|index| normalized[index..].to_owned())
        .unwrap_or_else(|| path.file_name().and_then(|value| value.to_str())
            .map(|name| format!("assets/scripts/{name}"))
            .unwrap_or_else(|| "assets/scripts/script.lua".into()))
}
