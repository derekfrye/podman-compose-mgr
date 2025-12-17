use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};

use crate::tui::app::DockerfileNameEntry;

pub(crate) fn draw_dockerfile_modal(
    frame: &mut Frame,
    full_area: Rect,
    entries: &[DockerfileNameEntry],
    selected_idx: usize,
    error: Option<&str>,
) {
    let base_height: u16 = 8;
    let rows_height = u16::try_from(entries.len()).unwrap_or(0);
    let error_height = if error.is_some() { 2 } else { 0 };
    let height = base_height
        .saturating_add(rows_height)
        .saturating_add(error_height);
    let width: u16 = 84;
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

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(error_height.max(1)),
        ])
        .split(area);

    let instructions = Paragraph::new(
        "Edit image names for Dockerfile builds.\nArrows move, type to edit, Enter=confirm, Esc=cancel.",
    )
    .block(Block::default().title("Image names").borders(Borders::ALL));

    let mut table_state = TableState::default();
    if !entries.is_empty() {
        table_state.select(Some(selected_idx.min(entries.len().saturating_sub(1))));
    }

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            let marker = if idx == selected_idx { ">" } else { " " };
            let cursor_pos = entry.cursor.min(entry.image_name.len());
            let image_text = if idx == selected_idx {
                let (left, right) = entry.image_name.split_at(cursor_pos);
                format!("{left}█{right}")
            } else {
                entry.image_name.clone()
            };
            let is_unknown = entry.image_name.trim().is_empty()
                || entry.image_name.trim().eq_ignore_ascii_case("unknown");
            let style = if is_unknown {
                Style::default().fg(Color::Red)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(marker),
                Cell::from(entry.dockerfile_name.clone()),
                Cell::from(image_text),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Percentage(35),
            Constraint::Percentage(63),
        ],
    )
    .header(
        Row::new(["", "Dockerfile", "Image name"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::ALL))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let error_line = error.unwrap_or("");
    let error_widget = Paragraph::new(error_line).style(Style::default().fg(Color::Red));

    frame.render_widget(Clear, area);
    frame.render_widget(instructions, chunks[0]);
    frame.render_stateful_widget(table, chunks[1], &mut table_state);
    frame.render_widget(error_widget, chunks[2]);
}
