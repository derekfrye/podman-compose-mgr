use crate::Args;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::app::{App, ModalState, UiState, ViewMode};

mod common;
mod list;
mod modals;
mod rebuild;

pub fn draw(frame: &mut Frame, app: &mut App, args: &Args) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(frame.area());

    let mut title_text = app.title.clone();
    if let ViewMode::ByFolderThenImage = app.view_mode {
        let rel = if app.current_path.is_empty() {
            ".".to_string()
        } else {
            format!("./{}", app.current_path.join("/"))
        };
        title_text = format!("{}  —  Folder: {}", app.title, rel);
    }
    let title = Paragraph::new(Line::from(vec![Span::styled(
        title_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    match app.state {
        UiState::Scanning => list::draw_scanning(frame, chunks[1], app, args),
        UiState::Ready => {
            list::draw_table(frame, chunks[1], app);
            list::draw_help_overlay(frame, frame.area());
        }
        UiState::Rebuilding => rebuild::draw_rebuild(frame, chunks[1], app),
    }

    match app.modal.clone() {
        Some(ModalState::ViewPicker { selected_idx }) => {
            modals::draw_view_picker(frame, frame.area(), selected_idx, app.view_mode);
        }
        Some(ModalState::WorkQueue { selected_idx }) => {
            modals::draw_work_queue(frame, frame.area(), app, selected_idx);
        }
        Some(ModalState::DockerfileNameEdit {
            entries,
            selected_idx,
            error,
        }) => {
            modals::draw_dockerfile_modal(
                frame,
                frame.area(),
                &entries,
                selected_idx,
                error.as_deref(),
            );
        }
        Some(ModalState::ExportLog { input, error }) => {
            modals::draw_export_modal(frame, frame.area(), &input, error.as_deref());
        }
        None => {}
    }
}
