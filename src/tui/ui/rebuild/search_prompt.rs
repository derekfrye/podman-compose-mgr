use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::tui::app::{SearchDirection, SearchState};

pub(super) fn search_prompt_visible(search: &SearchState) -> bool {
    search.editing || search.error.is_some() || search.has_query()
}

pub(super) fn draw_search_prompt(frame: &mut Frame, area: Rect, search: &SearchState) {
    frame.render_widget(Clear, area);
    let line = build_search_prompt_line(search);
    let widget = Paragraph::new(line);
    frame.render_widget(widget, area);
}

fn build_search_prompt_line(search: &SearchState) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let prefix = match search.direction {
        SearchDirection::Forward => "/",
        SearchDirection::Backward => "?",
    };
    spans.push(Span::styled(
        prefix.to_string(),
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));

    if !search.query.is_empty() {
        spans.push(Span::styled(
            search.query.clone(),
            Style::default().fg(Color::Yellow),
        ));
    }

    if search.editing {
        spans.push(Span::styled(
            "▏".to_string(),
            Style::default().fg(Color::Yellow),
        ));
    }

    if let Some(err) = &search.error {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("regex error: {err}"),
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        ));
        return Line::from(spans);
    }

    if search.has_query() {
        spans.push(Span::raw("  "));
        if search.matches.is_empty() {
            spans.push(Span::styled(
                "0 matches".to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        } else if let Some(active) = search.active {
            spans.push(Span::styled(
                format!("match {}/{}", active + 1, search.matches.len()),
                Style::default().fg(Color::Gray),
            ));
            spans.push(Span::raw("  n/N navigate"));
        } else {
            spans.push(Span::styled(
                format!("{} matches", search.matches.len()),
                Style::default().fg(Color::Gray),
            ));
        }
    } else if search.editing {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            "enter regex (Esc to cancel)",
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}
