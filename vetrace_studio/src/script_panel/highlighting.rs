use super::*;

pub(super) fn highlighted_layout_job(
    source: &str,
    highlights: &[HighlightSpan],
    diagnostics: &[ScriptDiagnostic],
    active_line: usize,
) -> egui::text::LayoutJob {
    let mut boundaries = vec![0usize, source.len()];
    for span in highlights {
        boundaries.push(span.range.start.min(source.len()));
        boundaries.push(span.range.end.min(source.len()));
    }
    for diagnostic in diagnostics {
        boundaries.push(diagnostic.range.start.min(source.len()));
        boundaries.push(diagnostic.range.end.min(source.len()));
    }
    let active = line_byte_range(source, active_line);
    boundaries.push(active.start);
    boundaries.push(active.end);
    boundaries.retain(|offset| source.is_char_boundary(*offset));
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut job = egui::text::LayoutJob::default();
    for pair in boundaries.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start == end { continue; }
        let kind = highlights.iter()
            .find(|span| span.range.start <= start && span.range.end >= end)
            .map(|span| span.kind)
            .unwrap_or(HighlightKind::Plain);
        let diagnostic = diagnostics.iter()
            .find(|diagnostic| diagnostic.range.start < end && diagnostic.range.end > start);
        let mut format = egui::TextFormat {
            font_id: egui::FontId::monospace(14.0),
            color: highlight_color(kind),
            ..Default::default()
        };
        if start >= active.start && end <= active.end {
            format.background = egui::Color32::from_rgba_unmultiplied(70, 85, 105, 42);
        }
        if let Some(diagnostic) = diagnostic {
            format.underline = egui::Stroke::new(
                1.5,
                match diagnostic.severity {
                    DiagnosticSeverity::Error => egui::Color32::from_rgb(245, 75, 75),
                    DiagnosticSeverity::Warning => egui::Color32::from_rgb(235, 180, 70),
                    DiagnosticSeverity::Information => egui::Color32::from_rgb(90, 155, 235),
                    DiagnosticSeverity::Hint => egui::Color32::from_rgb(120, 190, 170),
                },
            );
        }
        job.append(&source[start..end], 0.0, format);
    }
    job
}

pub(super) fn highlight_color(kind: HighlightKind) -> egui::Color32 {
    match kind {
        HighlightKind::Plain => egui::Color32::from_rgb(220, 224, 230),
        HighlightKind::Keyword => egui::Color32::from_rgb(205, 120, 245),
        HighlightKind::Builtin => egui::Color32::from_rgb(90, 190, 235),
        HighlightKind::Type => egui::Color32::from_rgb(90, 210, 175),
        HighlightKind::Function => egui::Color32::from_rgb(245, 205, 105),
        HighlightKind::Property => egui::Color32::from_rgb(125, 205, 245),
        HighlightKind::Number => egui::Color32::from_rgb(245, 150, 105),
        HighlightKind::String => egui::Color32::from_rgb(150, 210, 120),
        HighlightKind::Comment => egui::Color32::from_rgb(115, 130, 120),
        HighlightKind::Operator => egui::Color32::from_rgb(235, 125, 150),
        HighlightKind::Punctuation => egui::Color32::from_rgb(190, 195, 205),
        HighlightKind::Error => egui::Color32::from_rgb(245, 75, 75),
    }
}

pub(super) fn char_index_to_byte(source: &str, character_index: usize) -> usize {
    source.char_indices().nth(character_index).map(|(index, _)| index).unwrap_or(source.len())
}

pub(super) fn line_for_offset(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())].bytes().filter(|byte| *byte == b'\n').count() + 1
}

pub(super) fn byte_column(source: &str, offset: usize) -> usize {
    let offset = offset.min(source.len());
    let start = source[..offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
    source[start..offset].chars().count() + 1
}

pub(super) fn line_byte_range(source: &str, line: usize) -> TextRange {
    vetrace_script_editor::line_range(source, line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_helpers_handle_unicode() {
        assert_eq!(char_index_to_byte("aé", 2), 3);
        assert_eq!(line_for_offset("one\ntwo", 5), 2);
        assert_eq!(byte_column("one\ntwo", 5), 2);
    }
}
