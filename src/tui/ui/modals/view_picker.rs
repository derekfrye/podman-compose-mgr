use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::tui::app::ViewMode;

pub(crate) fn draw_view_picker(
    frame: &mut Frame,
    full_area: Rect,
    selected_idx: usize,
    current: ViewMode,
) {
    let height: u16 = 10;
    let width: u16 = 40;
    let width_final = width.min(full_area.width);
    let height_final = height.min(full_area.height);
    let left = full_area.x + (full_area.width.saturating_sub(width_final)) / 2;
    let top = full_area.y + (full_area.height.saturating_sub(height_final)) / 2;
    let area = Rect {
        x: left,
        y: top,
        width: width_final,
        height: height_final,
    };

    let items = [
        ('c', "List by container runtime name", ViewMode::ByContainer),
        ('i', "List by image", ViewMode::ByImage),
        ('f', "List by folder, then image", ViewMode::ByFolderThenImage),
        ('d', "List by Dockerfile", ViewMode::ByDockerfile),
        ('m', "List by Makefile", ViewMode::ByMakefile),
    ];
    let mut lines: Vec<Line> = Vec::new();
    for (i, (hotkey, label, mode)) in items.iter().enumerate() {
        let active = if *mode == current { "▶" } else { " " };
        let marker = if i == selected_idx { ">" } else { " " };
        let mut spans = vec![
            Span::styled(
                marker,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(active, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
        ];
        spans.push(Span::raw(*label));
        spans.push(Span::raw(" ("));
        spans.push(Span::styled(
            hotkey.to_string(),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(")"));
        lines.push(Line::from(spans));
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .title("View Options (Enter=select, Esc=close)")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(widget, area);
}
