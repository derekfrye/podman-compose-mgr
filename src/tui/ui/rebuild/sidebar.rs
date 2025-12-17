use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::{RebuildState, RebuildStatus};

use crate::tui::ui::common::styled_key;

pub(super) fn draw_rebuild_sidebar(
    frame: &mut Frame,
    area: ratatui::prelude::Rect,
    rebuild: &RebuildState,
) {
    let total = rebuild.jobs.len();
    let active = rebuild.active_idx + 1;
    let mut lines: Vec<Line> = Vec::new();

    frame.render_widget(Clear, area);

    lines.push(Line::from(format!("Job: {active}/{total}")));
    if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
        lines.push(Line::from(format!("Status: {}", format_status(job.status))));
        lines.push(Line::from(format!("Image: {}", job.image)));
        if let Some(container) = &job.container {
            lines.push(Line::from(format!("Container: {container}")));
        }
        lines.push(Line::from(format!("Source: {}", job.source_dir.display())));
        if let Some(err) = &job.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Error",
                Style::default().fg(Color::LightRed),
            )]));
            lines.push(Line::from(err.clone()));
        }
    } else {
        lines.push(Line::from("Status: —"));
    }
    lines.push(Line::from(""));
    lines.extend(legend_lines());

    let sidebar = Paragraph::new(lines)
        .block(Block::default().title("Legend").borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    frame.render_widget(sidebar, area);
}

fn format_status(status: RebuildStatus) -> &'static str {
    match status {
        RebuildStatus::Pending => "Pending",
        RebuildStatus::Running => "Running",
        RebuildStatus::Succeeded => "Done",
        RebuildStatus::Failed => "Failed",
    }
}

fn legend_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![styled_key("w", Color::Cyan), Span::raw(" Work queue")]),
        Line::from(vec![
            styled_key("e", Color::Green),
            Span::raw(" Export log"),
        ]),
        Line::from(vec![
            styled_key("↑/↓/←/→", Color::Yellow),
            Span::raw(" Scroll"),
        ]),
        Line::from(vec![
            styled_key("PgUp/PgDn", Color::Yellow),
            Span::raw(" Page scroll"),
        ]),
        Line::from(vec![
            styled_key("Home", Color::Yellow),
            Span::raw(" Goto top"),
        ]),
        Line::from(vec![
            styled_key("End", Color::Yellow),
            Span::raw(" Goto end"),
        ]),
        Line::from(vec![
            styled_key("/ ?", Color::Green),
            Span::raw(" Search (regex)"),
        ]),
        Line::from(vec![
            styled_key("n/N", Color::Green),
            Span::raw(" Next/prev match"),
        ]),
        Line::from(vec![
            styled_key("esc", Color::Magenta),
            Span::raw(" Back to list"),
        ]),
        Line::from(vec![styled_key("q", Color::Red), Span::raw(" Quit")]),
    ]
}
