use crate::Args;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::App;

pub fn draw(f: &mut Frame, app: &App, args: &Args) {
    // Create a layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),  // Title
                Constraint::Min(3),     // Main content
                Constraint::Length(3),  // Footer
            ]
            .as_ref(),
        )
        .split(f.area());

    // Title
    let title = Paragraph::new(Line::from(vec![Span::styled(
        &app.title,
        Style::default().fg(Color::Cyan),
    )]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Main content - display rebuild information
    let content = vec![
        Line::from("Press 'q' to quit"),
        Line::from(""),
        Line::from(format!("Path: {}", args.path.display())),
        Line::from(format!("Mode: {:?}", args.mode)),
    ];

    let content_widget = Paragraph::new(content)
        .block(Block::default().title("Info").borders(Borders::ALL));
    f.render_widget(content_widget, chunks[1]);

    // Footer
    let footer = Paragraph::new("Podman Compose Manager TUI")
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(footer, chunks[2]);
}