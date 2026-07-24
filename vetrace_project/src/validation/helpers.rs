use super::*;
use std::collections::BTreeSet;

pub(super) fn validate_required_text(
    report: &mut ValidationReport,
    field: &str,
    value: &str,
    max_len: usize,
) {
    if value.trim().is_empty() {
        report.push(ValidationIssue::error(
            "required_text",
            Some(field.to_owned()),
            "value cannot be empty",
        ));
    } else if value.chars().count() > max_len {
        report.push(ValidationIssue::error(
            "text_too_long",
            Some(field.to_owned()),
            format!("value cannot exceed {max_len} characters"),
        ));
    }
}

pub(super) fn looks_like_semver(value: &str) -> bool {
    let core = value.split_once('+').map_or(value, |(core, _)| core);
    let core = core.split_once('-').map_or(core, |(core, _)| core);
    let mut parts = core.split('.');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(a), Some(b), Some(c), None)
            if !a.is_empty()
                && !b.is_empty()
                && !c.is_empty()
                && a.chars().all(|ch| ch.is_ascii_digit())
                && b.chars().all(|ch| ch.is_ascii_digit())
                && c.chars().all(|ch| ch.is_ascii_digit())
    )
}

pub(super) fn valid_action_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|ch| {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'
        })
}

pub(super) fn warn_duplicates(
    report: &mut ValidationReport,
    field: &str,
    values: &[String],
) {
    let mut unique = BTreeSet::new();
    for value in values {
        if !unique.insert(value) {
            report.push(ValidationIssue::warning(
                "input_binding_duplicate",
                Some(field.to_owned()),
                format!("binding '{value}' is listed more than once"),
            ));
        }
    }
}
