use vetrace_script_editor::{HighlightKind, HighlightSpan, TextRange};

use crate::{BUILTINS, KEYWORDS};

pub(crate) fn tokenize(source: &str) -> Vec<HighlightSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        let start = index;
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if byte == b'-' && bytes.get(index + 1) == Some(&b'-') {
            index += 2;
            if bytes.get(index) == Some(&b'[') && bytes.get(index + 1) == Some(&b'[') {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b']' && bytes[index + 1] == b']') {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
            } else {
                while index < bytes.len() && bytes[index] != b'\n' { index += 1; }
            }
            spans.push(span(start, index, HighlightKind::Comment));
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            let quote = byte;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            spans.push(span(start, index, HighlightKind::String));
            continue;
        }
        if byte == b'[' && bytes.get(index + 1) == Some(&b'[') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b']' && bytes[index + 1] == b']') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            spans.push(span(start, index, HighlightKind::String));
            continue;
        }
        if byte.is_ascii_digit() || (byte == b'.' && bytes.get(index + 1).is_some_and(u8::is_ascii_digit)) {
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'.' | b'_' | b'+' | b'-'))
            {
                index += 1;
            }
            spans.push(span(start, index, HighlightKind::Number));
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            index += 1;
            while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
                index += 1;
            }
            let word = &source[start..index];
            let kind = if KEYWORDS.contains(&word) {
                HighlightKind::Keyword
            } else if BUILTINS.contains(&word) {
                HighlightKind::Builtin
            } else if next_non_whitespace(bytes, index) == Some(b'(') {
                HighlightKind::Function
            } else if previous_non_whitespace(bytes, start) == Some(b'.') || previous_non_whitespace(bytes, start) == Some(b':') {
                HighlightKind::Property
            } else {
                HighlightKind::Plain
            };
            if kind != HighlightKind::Plain { spans.push(span(start, index, kind)); }
            continue;
        }
        let kind = if matches!(byte, b'+' | b'-' | b'*' | b'/' | b'%' | b'^' | b'#' | b'=' | b'<' | b'>' | b'~') {
            HighlightKind::Operator
        } else {
            HighlightKind::Punctuation
        };
        index += 1;
        if index < bytes.len() && matches!((byte, bytes[index]), (b'=', b'=') | (b'~', b'=') | (b'<', b'=') | (b'>', b'=') | (b'.', b'.') | (b':', b':')) {
            index += 1;
            if byte == b'.' && bytes.get(index) == Some(&b'.') { index += 1; }
        }
        spans.push(span(start, index, kind));
    }
    spans
}

fn span(start: usize, end: usize, kind: HighlightKind) -> HighlightSpan {
    HighlightSpan { range: TextRange::new(start, end), kind }
}

fn next_non_whitespace(bytes: &[u8], mut index: usize) -> Option<u8> {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) { index += 1; }
    bytes.get(index).copied()
}

fn previous_non_whitespace(bytes: &[u8], mut index: usize) -> Option<u8> {
    while index > 0 {
        index -= 1;
        if !bytes[index].is_ascii_whitespace() { return Some(bytes[index]); }
    }
    None
}
