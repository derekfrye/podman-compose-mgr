use crate::Args;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
        Wrap,
    },
};

use crate::tui::app::{
    App, ItemRow, ModalState, OutputStream, RebuildState, RebuildStatus, UiState, ViewMode,
};

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
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Main
    match app.state {
        UiState::Scanning => draw_scanning(frame, chunks[1], app, args),
        UiState::Ready => {
            draw_table(frame, chunks[1], app);
            draw_help_overlay(frame, frame.area());
        }
        UiState::Rebuilding => draw_rebuild(frame, chunks[1], app),
    }

    // Modal overlays (draw last)
    match app.modal.clone() {
        Some(ModalState::ViewPicker { selected_idx }) => {
            draw_view_picker(frame, frame.area(), selected_idx, app.view_mode);
        }
        Some(ModalState::WorkQueue { selected_idx }) => {
            draw_work_queue(frame, frame.area(), app, selected_idx)
        }
        None => {}
    }
}

fn draw_scanning(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App, args: &Args) {
    let spinner = super::app::SPINNER_FRAMES[app.spinner_idx];
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

fn draw_table(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    let (header, widths) = match app.view_mode {
        ViewMode::ByContainer => (
            Row::new([
                Cell::from("Select"),
                Cell::from("Container"),
                Cell::from("Image"),
            ])
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

fn draw_rebuild(frame: &mut Frame, area: ratatui::prelude::Rect, app: &App) {
    let Some(rebuild) = &app.rebuild else {
        let empty = Paragraph::new("No rebuild jobs queued")
            .block(Block::default().title("Rebuild").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    let pane = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(40), Constraint::Length(24)])
        .split(area);

    draw_rebuild_output(frame, pane[0], rebuild);
    draw_rebuild_sidebar(frame, pane[1], rebuild);
}

fn draw_rebuild_output(frame: &mut Frame, area: ratatui::prelude::Rect, rebuild: &RebuildState) {
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        let empty = Paragraph::new("Waiting for jobs...")
            .block(Block::default().title("Output").borders(Borders::ALL));
        frame.render_widget(empty, area);
        return;
    };

    let header = match &job.container {
        Some(container) => format!("{} ({container})", job.image),
        None => job.image.clone(),
    };

    let lines: Vec<Line> = job
        .output
        .iter()
        .map(|entry| match entry.stream {
            OutputStream::Stdout => Line::from(vec![Span::raw(entry.text.clone())]),
            OutputStream::Stderr => Line::from(vec![Span::styled(
                entry.text.clone(),
                Style::default().fg(Color::LightRed),
            )]),
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(header).borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((rebuild.scroll_y, rebuild.scroll_x));

    frame.render_widget(paragraph, area);
}

fn draw_rebuild_sidebar(frame: &mut Frame, area: ratatui::prelude::Rect, rebuild: &RebuildState) {
    let total = rebuild.jobs.len();
    let active = rebuild.active_idx + 1;
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(format!("Job: {active}/{total}")));
    if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
        lines.push(Line::from(format!("Status: {}", format_status(job.status))));
        lines.push(Line::from(format!("Image: {}", job.image)));
        if let Some(container) = &job.container {
            lines.push(Line::from(format!("Container: {container}")));
        }
        lines.push(Line::from(format!("Source: {}", job.source_dir.display())));
        if let Some(err) = &job.error {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "Error",
                Style::default().fg(Color::LightRed),
            )]));
            lines.push(Line::from(err.clone()));
        }
    } else {
        lines.push(Line::from("Status: —"));
    }
    lines.push(Line::from(""));
    lines.extend(legend_lines());

    let sidebar = Paragraph::new(lines)
        .block(Block::default().title("Legend").borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    frame.render_widget(sidebar, area);
}

fn legend_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![styled_key("q", Color::Red), Span::raw(" Quit")]),
        Line::from(vec![styled_key("w", Color::Cyan), Span::raw(" Work queue")]),
        Line::from(vec![styled_key("j/k", Color::Yellow), Span::raw(" Scroll")]),
        Line::from(vec![
            styled_key("f/b", Color::Yellow),
            Span::raw(" Page scroll"),
        ]),
        Line::from(vec![
            styled_key("h/l", Color::Yellow),
            Span::raw(" Horizontal"),
        ]),
        Line::from(vec![
            styled_key("esc", Color::Magenta),
            Span::raw(" Back to list"),
        ]),
        Line::from(vec![
            styled_key("a", Color::Green),
            Span::raw(" Toggle all"),
        ]),
    ]
}

fn format_status(status: RebuildStatus) -> &'static str {
    match status {
        RebuildStatus::Pending => "Pending",
        RebuildStatus::Running => "Running",
        RebuildStatus::Succeeded => "Done",
        RebuildStatus::Failed => "Failed",
    }
}

fn draw_work_queue(frame: &mut Frame, full_area: Rect, app: &App, selected_idx: usize) {
    let Some(rebuild) = &app.rebuild else {
        return;
    };

    let height: u16 = (rebuild.jobs.len().min(12) as u16).saturating_add(4);
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

fn row_for_item<'a>(app: &'a App, it: &'a ItemRow) -> Row<'a> {
    let checkbox = if it.checked { "[x]" } else { "[ ]" };
    match app.view_mode {
        ViewMode::ByContainer => {
            let container = it.container.clone().unwrap_or_else(|| "—".to_string());
            Row::new([
                Cell::from(checkbox),
                Cell::from(container),
                Cell::from(it.image.clone()),
            ])
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
                    ViewMode::ByImage | ViewMode::ByFolderThenImage => {
                        rows.push(Row::new([Cell::from(""), Cell::from(indented)]));
                    }
                }
                visual_idx += 1;
            }
        }
    }
    (rows, selected_visual_idx)
}

fn draw_help_overlay(frame: &mut Frame, full_area: Rect) {
    let lines = help_overlay_lines();
    let area = help_overlay_area(full_area);
    let widget = Paragraph::new(lines).block(help_overlay_block());

    frame.render_widget(Clear, area);
    frame.render_widget(widget, area);
}

fn help_overlay_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            styled_key("↑/↓", Color::Yellow),
            Span::raw(" scroll   "),
            styled_key("←/→", Color::Yellow),
            Span::raw(" details   "),
            styled_key("[space]", Color::Green),
            Span::raw(" select   "),
            styled_key("q", Color::Red),
            Span::raw(" quit"),
        ]),
        Line::from(vec![styled_key("v", Color::Cyan), Span::raw(" View")]),
    ]
}

fn styled_key(content: &'static str, color: Color) -> Span<'static> {
    Span::styled(
        content,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
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

fn draw_view_picker(frame: &mut Frame, full_area: Rect, selected_idx: usize, current: ViewMode) {
    // Popup size
    let height: u16 = 8; // title + 3 items + padding
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
            Span::styled(
                marker,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(active, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::raw(*label),
        ]);
        lines.push(styled);
    }

    let widget = Paragraph::new(lines).block(
        Block::default()
            .title("View Options (Enter=select, Esc=close)")
            .borders(Borders::ALL),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(widget, area);
}
