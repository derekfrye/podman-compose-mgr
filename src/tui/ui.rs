use crate::Args;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};

use super::app::{App, UiState, ItemRow, ModalState, ViewMode};

pub fn draw(frame: &mut Frame, app: &App, args: &Args) {
    // Layout: title | main (draw help as overlay later)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(frame.area());

    // Title
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
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Main
    match app.state {
        UiState::Scanning => draw_scanning(frame, chunks[1], app, args),
        UiState::Ready => draw_table(frame, chunks[1], app),
    }

    // Help overlay (always on top)
    draw_help_overlay(frame, frame.area());

    // Modal overlays (draw last)
    if let Some(ModalState::ViewPicker { selected_idx }) = app.modal.clone() {
        draw_view_picker(frame, frame.area(), selected_idx, app.view_mode);
    }
}

fn draw_scanning(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App, args: &Args) {
    let spinner = super::app::SPINNER_FRAMES[app.spinner_idx];
    let line = Line::from(vec![
        Span::styled(spinner, Style::default().fg(Color::Yellow)),
        Span::raw("  Scanning "),
        Span::styled(args.path.display().to_string(), Style::default().fg(Color::White)),
        Span::raw(" for images..."),
    ]);

    let widget = Paragraph::new(line)
        .block(Block::default().title("Status").borders(Borders::ALL));
    frame.render_widget(widget, area);
}

fn draw_table(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    let (header, widths) = match app.view_mode {
        ViewMode::ByContainer => (
            Row::new([Cell::from("Select"), Cell::from("Container"), Cell::from("Image")])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            vec![
                Constraint::Length(6),
                Constraint::Percentage(35),
                Constraint::Percentage(59),
            ],
        ),
        ViewMode::ByImage => (
            Row::new([Cell::from("Select"), Cell::from("Image")])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            vec![Constraint::Length(6), Constraint::Percentage(94)],
        ),
        ViewMode::ByFolderThenImage => (
            Row::new([Cell::from("Select"), Cell::from("Name")])
                .style(Style::default().add_modifier(Modifier::BOLD)),
            vec![Constraint::Length(6), Constraint::Percentage(94)],
        ),
    };

    let (rows, selected_visual_idx) = build_rows_with_expansion(app);

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title("Images").borders(Borders::ALL))
        .row_highlight_style(Style::default().bg(Color::Blue).fg(Color::Black))
        .highlight_symbol("▶ ");

    // Render table with selection highlight
    let mut state = TableState::default();
    if !app.rows.is_empty() {
        state.select(Some(selected_visual_idx));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn row_for_item<'a>(app: &'a App, it: &'a ItemRow) -> Row<'a> {
    let checkbox = if it.checked { "[x]" } else { "[ ]" };
    match app.view_mode {
        ViewMode::ByContainer => {
            let container = it.container.clone().unwrap_or_else(|| "—".to_string());
            Row::new([Cell::from(checkbox), Cell::from(container), Cell::from(it.image.clone())])
        }
        ViewMode::ByImage => Row::new([Cell::from(checkbox), Cell::from(it.image.clone())]),
        ViewMode::ByFolderThenImage => {
            if it.is_dir {
                let name: String = it.dir_name.clone().unwrap_or_default();
                Row::new([Cell::from(checkbox), Cell::from(format!("📁 {name}"))])
            } else {
                Row::new([Cell::from(checkbox), Cell::from(it.image.clone())])
            }
        }
    }
}

fn build_rows_with_expansion(app: &App) -> (Vec<Row<'_>>, usize) {
    let mut rows: Vec<Row> = Vec::new();
    let mut visual_idx = 0usize;
    let mut selected_visual_idx = 0usize;
    for (i, it) in app.rows.iter().enumerate() {
        if i == app.selected {
            selected_visual_idx = visual_idx;
        }
        rows.push(row_for_item(app, it));
        visual_idx += 1;
        if it.expanded {
            // add detail rows indented under the item
            for line in &it.details {
                let indented = format!("  {line}");
                match app.view_mode {
                    ViewMode::ByContainer => rows.push(Row::new([
                        Cell::from(""),
                        Cell::from(indented),
                        Cell::from(""),
                    ])),
                    ViewMode::ByImage | ViewMode::ByFolderThenImage => rows.push(Row::new([Cell::from(""), Cell::from(indented)])),
                }
                visual_idx += 1;
            }
        }
    }
    (rows, selected_visual_idx)
}

fn draw_help_overlay(frame: &mut Frame, full_area: Rect) {
    // Compose help text with glyphs (two lines)
    let lines = vec![
        Line::from(vec![
            Span::styled("↑/↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" scroll   "),
            Span::styled("←/→", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" details   "),
            Span::styled("[space]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" select   "),
            Span::styled("q", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" quit"),
        ]),
        Line::from(vec![
            Span::styled("v", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" View"),
        ]),
    ];

    let widget = Paragraph::new(lines).block(Block::default().title("Keys").borders(Borders::ALL));

    // Size: 4 rows tall, width based on content
    let help_height: u16 = 4;
    // Make the overlay wide enough to include all labels
    let content_width: u16 = 55; // approximate width of the lines above inside borders
    let help_width: u16 = content_width + 2; // borders
    let width_final = help_width.min(full_area.width);
    let height_final = help_height.min(full_area.height);
    let left = full_area.x; // align to left side
    let top = full_area.y + full_area.height.saturating_sub(height_final);
    let area = Rect { x: left, y: top, width: width_final, height: height_final };

    // Clear and draw overlay last so it sits above content
    frame.render_widget(Clear, area);
    frame.render_widget(widget, area);
}

fn draw_view_picker(frame: &mut Frame, full_area: Rect, selected_idx: usize, current: ViewMode) {
    // Popup size
    let height: u16 = 8; // title + 3 items + padding
    let width: u16 = 40;
    let width_final = width.min(full_area.width);
    let height_final = height.min(full_area.height);
    let left = full_area.x + (full_area.width.saturating_sub(width_final)) / 2;
    let top = full_area.y + (full_area.height.saturating_sub(height_final)) / 2;
    let area = Rect { x: left, y: top, width: width_final, height: height_final };

    // Build item lines
    let items = [
        ("List by container runtime name", ViewMode::ByContainer),
        ("List by image", ViewMode::ByImage),
        ("List by folder, then image", ViewMode::ByFolderThenImage),
    ];
    let mut lines: Vec<Line> = Vec::new();
    for (i, (label, mode)) in items.iter().enumerate() {
        let active = if *mode == current { "▶" } else { " " };
        let marker = if i == selected_idx { ">" } else { " " };
        let styled = Line::from(vec![
            Span::styled(marker, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(active, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::raw(*label),
        ]);
        lines.push(styled);
    }

    let widget = Paragraph::new(lines)
        .block(Block::default().title("View Options (Enter=select, Esc=close)").borders(Borders::ALL));

    frame.render_widget(Clear, area);
    frame.render_widget(widget, area);
}
