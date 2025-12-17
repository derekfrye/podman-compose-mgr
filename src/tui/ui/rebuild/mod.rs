use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::App;

mod output;
mod output_lines;
mod search_prompt;
mod sidebar;

pub fn draw_rebuild(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(rebuild) = app.rebuild.as_mut() else {
        let empty = Paragraph::new("No rebuild jobs queued")
            .block(Block::default().title("Rebuild").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    let pane = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(24)])
        .split(area);

    output::draw_rebuild_output(frame, pane[0], rebuild);
    sidebar::draw_rebuild_sidebar(frame, pane[1], rebuild);
}
