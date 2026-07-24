use super::types::{TextPosition, TextRange};

pub fn offset_to_position(source: &str, offset: usize) -> TextPosition {
    let offset = offset.min(source.len());
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, character) in source.char_indices() {
        if index >= offset { break; }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    TextPosition { line, column }
}

pub fn line_range(source: &str, one_based_line: usize) -> TextRange {
    let target = one_based_line.max(1);
    let mut line = 1usize;
    let mut start = 0usize;
    for (index, character) in source.char_indices() {
        if line == target { start = index; break; }
        if character == '\n' { line += 1; }
    }
    if target > line && source.is_empty() { return TextRange::new(0, 0); }
    if target > line { return TextRange::new(source.len(), source.len()); }
    let end = source[start..].find('\n').map(|relative| start + relative).unwrap_or(source.len());
    TextRange::new(start, end)
}
