use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::{Args, tui::app::App};

pub(crate) fn draw_scanning(frame: &mut Frame, area: Rect, app: &App, args: &Args) {
    let spinner = crate::tui::app::SPINNER_FRAMES[app.spinner_idx];
    let line = Line::from(vec![
        Span::styled(spinner, Style::default().fg(Color::Yellow)),
        Span::raw("  Scanning "),
        Span::styled(
            args.path.display().to_string(),
            Style::default().fg(Color::White),
        ),
        Span::raw(" for images..."),
    ]);

    let widget = Paragraph::new(line).block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(widget, area);
}
