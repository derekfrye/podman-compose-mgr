use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::tui::app::{OutputStream, RebuildJob, SearchState};

pub(super) fn build_output_lines(
    job: &RebuildJob,
    search_state: Option<&SearchState>,
    text_width: u16,
) -> (Vec<Line<'static>>, usize) {
    let mut lines: Vec<Line> = Vec::with_capacity(job.output.len());
    let mut max_line_width: usize = 0;
    for (line_idx, entry) in job.output.iter().enumerate() {
        let normalized = normalize_line(&entry.text, text_width);
        max_line_width = max_line_width.max(UnicodeWidthStr::width(normalized.as_str()));
        let line = build_line_with_search(&normalized, entry.stream, search_state, line_idx);
        lines.push(line);
    }
    (lines, max_line_width)
}

pub(super) fn count_digits(mut n: usize) -> usize {
    let mut digits = 1;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits
}

fn build_line_with_search(
    text: &str,
    stream: OutputStream,
    search: Option<&SearchState>,
    line_idx: usize,
) -> Line<'static> {
    let base_style = style_for_stream(stream);
    let mut spans: Vec<Span<'static>> = Vec::new();

    if let Some(search) = search
        && let Some(indices) = search.matches_for_line(line_idx)
    {
        let mut cursor = 0usize;
        for idx in indices {
            if let Some(hit) = search.matches.get(*idx) {
                let start = hit.start.min(text.len());
                let end = hit.end.min(text.len());
                if start > cursor {
                    spans.push(Span::styled(text[cursor..start].to_string(), base_style));
                }
                let highlight = highlight_style(base_style, search.active == Some(*idx));
                spans.push(Span::styled(text[start..end].to_string(), highlight));
                cursor = end;
            }
        }
        if cursor < text.len() {
            spans.push(Span::styled(text[cursor..].to_string(), base_style));
        }

        if !spans.is_empty() {
            return Line::from(spans);
        }
    }

    match stream {
        OutputStream::Stdout | OutputStream::Stderr => {
            Line::from(vec![Span::styled(text.to_string(), base_style)])
        }
    }
}

fn style_for_stream(stream: OutputStream) -> Style {
    match stream {
        OutputStream::Stdout => Style::default(),
        OutputStream::Stderr => Style::default().fg(Color::LightRed),
    }
}

fn highlight_style(base: Style, is_active: bool) -> Style {
    let highlight = Style::default()
        .bg(if is_active {
            Color::Yellow
        } else {
            Color::DarkGray
        })
        .fg(if is_active {
            Color::Black
        } else {
            Color::White
        })
        .add_modifier(Modifier::BOLD);
    base.patch(highlight)
}

fn normalize_line(text: &str, _content_width: u16) -> String {
    let segment = text.rsplit('\r').next().unwrap_or(text);

    segment.replace('\t', "    ")
}
