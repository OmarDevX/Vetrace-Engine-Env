pub fn humanize_identifier(name: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = true;
    let mut previous_lowercase = false;
    for character in name.chars() {
        if character == '_' || character == '-' || character == '.' {
            if !result.ends_with(' ') && !result.is_empty() { result.push(' '); }
            uppercase_next = true;
            previous_lowercase = false;
            continue;
        }
        if character.is_uppercase() && previous_lowercase && !result.ends_with(' ') {
            result.push(' ');
        }
        if uppercase_next {
            result.extend(character.to_uppercase());
            uppercase_next = false;
        } else {
            result.push(character);
        }
        previous_lowercase = character.is_lowercase();
    }
    result
}
