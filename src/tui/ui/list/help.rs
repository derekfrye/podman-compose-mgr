use ratatui::{
    Frame,
    layout::Rect,
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::super::common::styled_key;

pub(crate) fn draw_help_overlay(frame: &mut Frame, full_area: Rect) {
    let lines = help_overlay_lines();
    let area = help_overlay_area(full_area);
    let widget = Paragraph::new(lines).block(help_overlay_block());

    frame.render_widget(ratatui::widgets::Clear, area);
    frame.render_widget(widget, area);
}

fn help_overlay_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            styled_key("↑/↓", Color::Yellow),
            Span::raw(" scroll  "),
            styled_key("←/→", Color::Yellow),
            Span::raw(" details  "),
            styled_key("x/<space>", Color::Green),
            Span::raw(" select  "),
            styled_key("q", Color::Red),
            Span::raw("/"),
            styled_key("Esc", Color::Red),
            Span::raw(" quit"),
        ]),
        Line::from(vec![
            styled_key("r", Color::Green),
            Span::raw(" rebuild selected images  "),
            styled_key("j", Color::Green),
            Span::raw(" show rebuild jobs"),
        ]),
        Line::from(vec![styled_key("v", Color::Cyan), Span::raw(" View")]),
    ]
}

fn help_overlay_area(full_area: Rect) -> Rect {
    let help_height: u16 = 4;
    let content_width: u16 = 55;
    let help_width: u16 = content_width + 2;
    let width_final = help_width.min(full_area.width);
    let height_final = help_height.min(full_area.height);
    let left = full_area.x;
    let top = full_area.y + full_area.height.saturating_sub(height_final);

    Rect {
        x: left,
        y: top,
        width: width_final,
        height: height_final,
    }
}

fn help_overlay_block() -> Block<'static> {
    Block::default().title("Keys").borders(Borders::ALL)
}
