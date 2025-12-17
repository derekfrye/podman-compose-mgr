use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::tui::app::{App, RebuildStatus};

pub(crate) fn draw_work_queue(frame: &mut Frame, full_area: Rect, app: &App, selected_idx: usize) {
    let Some(rebuild) = &app.rebuild else {
        return;
    };

    let height = u16::try_from(rebuild.jobs.len().min(12))
        .unwrap_or(u16::MAX)
        .saturating_add(4);
    let width: u16 = 48;
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

    let items: Vec<ListItem> = rebuild
        .jobs
        .iter()
        .enumerate()
        .map(|(idx, job)| {
            let prefix = if idx == rebuild.active_idx {
                "▶"
            } else {
                " "
            };
            let container = job
                .container
                .as_ref()
                .map_or(String::new(), |c| format!(" ({c})"));
            let line = format!("{prefix} {}{}", job.image, container);
            let style = match job.status {
                RebuildStatus::Pending => Style::default(),
                RebuildStatus::Running => Style::default().fg(Color::Yellow),
                RebuildStatus::Succeeded => Style::default().fg(Color::Green),
                RebuildStatus::Failed => Style::default().fg(Color::Red),
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Work Queue (Esc=close)")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(selected_idx));

    frame.render_widget(Clear, area);
    frame.render_stateful_widget(list, area, &mut state);
}
