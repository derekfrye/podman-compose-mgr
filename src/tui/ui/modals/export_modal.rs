use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub(crate) fn draw_export_modal(
    frame: &mut Frame,
    full_area: Rect,
    input: &str,
    error: Option<&str>,
) {
    let width: u16 = 72;
    let base_height: u16 = 6;
    let height = if error.is_some() {
        base_height + 2
    } else {
        base_height
    };
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

    let mut lines = vec![
        Line::from("Enter a filename to export the rebuild log:"),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Cyan)),
            Span::raw(input),
        ]),
        Line::from(""),
        Line::from("Esc cancels. Enter saves in the current working directory."),
    ];
    if let Some(err) = error {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            err,
            Style::default().fg(Color::LightRed),
        )]));
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .title("Export Rebuild Output")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(widget, area);
}
