use vetrace_script_editor::{
    line_range, LanguageError, ScriptSymbol, SignatureHelp, SymbolKind, TextEdit, TextRange,
};

use crate::{BUILTINS, KEYWORDS};

#[derive(Clone, Debug)]
struct IdentifierToken {
    name: String,
    range: TextRange,
}

pub(crate) fn lua_symbols(source: &str) -> Vec<ScriptSymbol> {
    let masked = mask_lua_literals(source);
    let tokens = identifier_tokens(&masked);
    let mut symbols = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if KEYWORDS.contains(&token.name.as_str()) || BUILTINS.contains(&token.name.as_str()) {
            continue;
        }
        let previous = index.checked_sub(1).and_then(|i| tokens.get(i));
        let next = tokens.get(index + 1);
        let line = line_range(&masked, vetrace_script_editor::offset_to_position(&masked, token.range.start).line);
        let before = &masked[line.start..token.range.start];
        let after = &masked[token.range.end..line.end];
        let kind = if previous.is_some_and(|previous| previous.name == "function")
            || after.trim_start().starts_with("= function")
        {
            Some(SymbolKind::Function)
        } else if previous.is_some_and(|previous| previous.name == "local")
            && !next.is_some_and(|next| next.name == "function")
        {
            Some(SymbolKind::Local)
        } else if is_function_parameter(&masked, token.range.start) {
            Some(SymbolKind::Parameter)
        } else if before.trim_end().ends_with("properties = {") || after.trim_start().starts_with("=") && inside_properties_table(&masked, token.range.start) {
            Some(SymbolKind::Property)
        } else {
            None
        };
        if let Some(kind) = kind {
            if !symbols.iter().any(|symbol: &ScriptSymbol| symbol.selection_range == token.range) {
                symbols.push(ScriptSymbol {
                    name: token.name.clone(),
                    kind,
                    range: line,
                    selection_range: token.range,
                });
            }
        }
    }
    symbols.sort_by_key(|symbol| symbol.selection_range.start);
    symbols
}

pub(crate) fn lua_definition(source: &str, cursor_byte: usize) -> Option<TextRange> {
    let name = identifier_at(source, cursor_byte)?.name;
    let symbols = lua_symbols(source);
    symbols.iter()
        .filter(|symbol| symbol.name == name && symbol.selection_range.start <= cursor_byte)
        .max_by_key(|symbol| symbol.selection_range.start)
        .or_else(|| symbols.iter().find(|symbol| symbol.name == name))
        .map(|symbol| symbol.selection_range)
}

pub(crate) fn lua_references(source: &str, cursor_byte: usize) -> Vec<TextRange> {
    let Some(identifier) = identifier_at(source, cursor_byte) else { return Vec::new(); };
    identifier_tokens(&mask_lua_literals(source)).into_iter()
        .filter(|token| token.name == identifier.name)
        .map(|token| token.range)
        .collect()
}

pub(crate) fn lua_rename_edits(
    source: &str,
    cursor_byte: usize,
    new_name: &str,
) -> Result<Vec<TextEdit>, LanguageError> {
    if !is_lua_identifier(new_name) || KEYWORDS.contains(&new_name) {
        return Err(LanguageError(format!("'{new_name}' is not a valid Lua identifier")));
    }
    let references = lua_references(source, cursor_byte);
    if references.is_empty() {
        return Err(LanguageError("place the cursor on a Lua symbol to rename it".into()));
    }
    Ok(references.into_iter().map(|range| TextEdit {
        range,
        replacement: new_name.to_owned(),
    }).collect())
}

pub(crate) fn lua_signature_help(source: &str, cursor_byte: usize) -> Option<SignatureHelp> {
    let masked = mask_lua_literals(source);
    let cursor = cursor_byte.min(masked.len());
    let bytes = masked.as_bytes();
    let mut depth = 0usize;
    let mut open = None;
    let mut index = cursor;
    while index > 0 {
        index -= 1;
        match bytes[index] {
            b')' => depth += 1,
            b'(' if depth == 0 => { open = Some(index); break; }
            b'(' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let open = open?;
    let mut start = open;
    while start > 0 {
        let byte = bytes[start - 1];
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':') {
            start -= 1;
        } else {
            break;
        }
    }
    let callee = masked[start..open].trim();
    if callee.is_empty() { return None; }
    let mut active_parameter = 0usize;
    let mut nested = 0usize;
    for byte in bytes[open + 1..cursor].iter().copied() {
        match byte {
            b'(' | b'{' | b'[' => nested += 1,
            b')' | b'}' | b']' => nested = nested.saturating_sub(1),
            b',' if nested == 0 => active_parameter += 1,
            _ => {}
        }
    }
    let (parameters, documentation) = builtin_signature(callee)
        .or_else(|| local_function_signature(&masked, callee))?;
    Some(SignatureHelp {
        label: format!("{callee}({})", parameters.join(", ")),
        parameters,
        active_parameter,
        documentation: Some(documentation),
    })
}

fn builtin_signature(name: &str) -> Option<(Vec<String>, String)> {
    let signature: &[&str] = match name {
        "Scene.instantiate" => &["scene_path", "position?"],
        "Scene.spawn" => &["name?"],
        "Scene.destroy" => &["entity"],
        "Physics.raycast" => &["origin", "direction", "max_distance?"],
        "Physics.apply_impulse" => &["entity", "impulse"],
        "Physics.set_velocity" => &["entity", "velocity"],
        "Physics.set_enabled" => &["entity", "enabled"],
        "Physics.is_enabled" => &["entity"],
        "Audio.play" => &["asset_path", "volume?"],
        "Audio.play_3d" => &["asset_path", "position", "volume?"],
        "Events.emit" => &["entity", "name", "payload?"],
        "Events.broadcast" => &["name", "payload?"],
        "Assets.read_text" => &["asset_path"],
        "Assets.exists" => &["asset_path"],
        "Modules.require" | "Modules.invalidate" | "Modules.is_loaded" => &["module_path"],
        "Input.action_down" | "Input.action_pressed" | "Input.action_released" => &["action"],
        "Debug.print" => &["value"],
        "print" => &["..."],
        _ => return None,
    };
    Some((signature.iter().map(|value| (*value).to_owned()).collect(), "Vetrace Lua API".into()))
}

fn local_function_signature(source: &str, name: &str) -> Option<(Vec<String>, String)> {
    for needle in [format!("function {name}("), format!("{name} = function(")] {
        let Some(found) = source.find(&needle) else { continue; };
        let start = found + needle.len();
        let Some(relative_end) = source[start..].find(')') else { continue; };
        let end = start + relative_end;
        let parameters = source[start..end].split(',')
            .map(str::trim)
            .filter(|parameter| !parameter.is_empty())
            .map(str::to_owned)
            .collect();
        return Some((parameters, "Project Lua function".into()));
    }
    None
}

fn identifier_at(source: &str, cursor_byte: usize) -> Option<IdentifierToken> {
    let cursor = cursor_byte.min(source.len());
    identifier_tokens(&mask_lua_literals(source)).into_iter().find(|token| {
        token.range.start <= cursor && cursor <= token.range.end
            || cursor > 0 && token.range.start < cursor && cursor <= token.range.end
    })
}

fn identifier_tokens(source: &str) -> Vec<IdentifierToken> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_') {
                index += 1;
            }
            tokens.push(IdentifierToken {
                name: source[start..index].to_owned(),
                range: TextRange::new(start, index),
            });
        } else {
            index += 1;
        }
    }
    tokens
}

pub(crate) fn mask_lua_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'-' && bytes.get(index + 1) == Some(&b'-') {
            let start = index;
            index += 2;
            if bytes.get(index) == Some(&b'[') && bytes.get(index + 1) == Some(&b'[') {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b']' && bytes[index + 1] == b']') { index += 1; }
                index = (index + 2).min(bytes.len());
            } else {
                while index < bytes.len() && bytes[index] != b'\n' { index += 1; }
            }
            for byte in &mut output[start..index] { if *byte != b'\n' { *byte = b' '; } }
        } else if matches!(bytes[index], b'\'' | b'"') {
            let start = index;
            let quote = bytes[index];
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' { index = (index + 2).min(bytes.len()); }
                else if bytes[index] == quote { index += 1; break; }
                else { index += 1; }
            }
            for byte in &mut output[start..index] { if *byte != b'\n' { *byte = b' '; } }
        } else if bytes[index] == b'[' && bytes.get(index + 1) == Some(&b'[') {
            let start = index;
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b']' && bytes[index + 1] == b']') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            for byte in &mut output[start..index] { if *byte != b'\n' { *byte = b' '; } }
        } else {
            index += 1;
        }
    }
    String::from_utf8(output).unwrap_or_else(|_| source.to_owned())
}

fn is_function_parameter(source: &str, offset: usize) -> bool {
    let line = line_range(source, vetrace_script_editor::offset_to_position(source, offset).line);
    let before = &source[line.start..offset];
    let Some(open) = before.rfind('(') else { return false; };
    let prefix = before[..open].trim_end();
    prefix.ends_with("function") || prefix.split_whitespace().last().is_some_and(|part| part.contains("function"))
        || prefix.rfind("function").is_some_and(|function| !prefix[function..].contains(')'))
}

fn inside_properties_table(source: &str, offset: usize) -> bool {
    let prefix = &source[..offset.min(source.len())];
    let Some(properties) = prefix.rfind("properties") else { return false; };
    let segment = &prefix[properties..];
    segment.bytes().filter(|byte| *byte == b'{').count() > segment.bytes().filter(|byte| *byte == b'}').count()
}

fn is_lua_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else { return false; };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}
