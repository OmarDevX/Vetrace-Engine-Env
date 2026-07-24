use crate::mask_lua_literals;

pub(crate) fn format_lua(source: &str) -> String {
    if source.is_empty() { return String::new(); }

    // Structural decisions use a same-length copy with comments and strings
    // blanked out. Keywords inside text such as "end" or comments therefore
    // cannot corrupt indentation, while the original source remains untouched.
    let masked = mask_lua_literals(source);
    let mut output = String::new();
    let mut indent = 0usize;
    for (raw_line, structural_line) in source.lines().zip(masked.lines()) {
        let trimmed = raw_line.trim();
        let structural = structural_line.trim();
        let first = structural.split_whitespace().next().unwrap_or("");
        let dedent = matches!(first, "end" | "else" | "elseif" | "until")
            || structural.starts_with('}');
        if dedent { indent = indent.saturating_sub(1); }
        if !trimmed.is_empty() {
            output.push_str(&"    ".repeat(indent));
            output.push_str(trimmed);
        }
        output.push('\n');
        let opens = structural.ends_with("then")
            || structural.ends_with(" do")
            || structural == "do"
            || structural.starts_with("function ")
            || structural.contains(" = function(")
            || structural == "repeat"
            || structural.ends_with('{');
        if opens || matches!(first, "else" | "elseif") { indent += 1; }
    }
    output
}
