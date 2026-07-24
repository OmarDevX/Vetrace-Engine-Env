use super::*;

pub fn parse_console_script_location(project_root: &Path, text: &str) -> Option<(PathBuf, usize)> {
    let diagnostic = parse_runtime_diagnostic(project_root, text)?;
    Some((diagnostic.path, diagnostic.line))
}

pub(super) fn parse_runtime_diagnostic(project_root: &Path, text: &str) -> Option<RuntimeScriptDiagnostic> {
    if let Some(rest) = text.strip_prefix("VETRACE_SCRIPT_DIAGNOSTIC\t") {
        let mut parts = rest.splitn(4, '\t');
        let relative = parts.next()?;
        let line = parts.next()?.parse::<usize>().ok()?.max(1);
        let column = parts.next()?.parse::<usize>().ok()?.max(1);
        let message = parts.next().unwrap_or("Lua error").to_owned();
        return Some(RuntimeScriptDiagnostic {
            path: project_root.join(relative),
            line,
            column,
            message,
        });
    }

    let script_start = text.find("assets/scripts/")?;
    let after = &text[script_start..];
    let lua_end = after.find(".lua")? + 4;
    let relative = &after[..lua_end];
    let remainder = &after[lua_end..];
    let line = parse_line_after_script(remainder).unwrap_or(1);
    Some(RuntimeScriptDiagnostic {
        path: project_root.join(relative),
        line,
        column: 1,
        message: text.to_owned(),
    })
}

pub(super) fn parse_line_after_script(text: &str) -> Option<usize> {
    for (index, character) in text.char_indices() {
        if character != ':' {
            continue;
        }

        let candidate = &text[index + character.len_utf8()..];
        let digits: String = candidate
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect();
        if let Ok(line) = digits.parse::<usize>() {
            return Some(line.max(1));
        }
    }
    None
}
