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

pub fn draw(frame: &mut Frame, app: &mut App, args: &Args) {
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
            draw_work_queue(frame, frame.area(), app, selected_idx);
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
            Row::new([
                Cell::from("Select"),
                Cell::from("Image"),
                Cell::from("Container(s)"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
            vec![
                Constraint::Length(6),
                Constraint::Percentage(50),
                Constraint::Percentage(44),
            ],
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
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    // Render table with selection highlight
    let mut state = TableState::default();
    if !app.rows.is_empty() {
        state.select(Some(selected_visual_idx));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_rebuild(frame: &mut Frame, area: ratatui::prelude::Rect, app: &mut App) {
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

    draw_rebuild_output(frame, pane[0], rebuild);
    draw_rebuild_sidebar(frame, pane[1], rebuild);
}

fn draw_rebuild_output(
    frame: &mut Frame,
    area: ratatui::prelude::Rect,
    rebuild: &mut RebuildState,
) {
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

    // Ensure no stale table rows remain when switching from the list view into the rebuild pane.
    frame.render_widget(Clear, area);
    let block = Block::default().title(header).borders(Borders::ALL);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if inner_area.width == 0 || inner_area.height == 0 {
        return;
    }

    let total_lines = job.output.len().max(1);
    let line_digits = count_digits(total_lines).max(3);
    let mut gutter_width = (line_digits + 1) as u16;
    if gutter_width >= inner_area.width {
        gutter_width = line_digits as u16;
        if gutter_width >= inner_area.width {
            gutter_width = 0;
        }
    }

    let mut text_area = inner_area;
    let mut number_area = None;
    if gutter_width > 0 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(gutter_width), Constraint::Min(1)])
            .split(inner_area);
        number_area = Some(split[0]);
        text_area = split[1];
    }

    if text_area.width == 0 || text_area.height == 0 {
        return;
    }

    let content_height = text_area.height.max(1);
    let content_width = text_area.width;
    rebuild.viewport_height = content_height;
    rebuild.viewport_width = content_width;

    let viewport = usize::from(content_height);
    let max_start = job.output.len().saturating_sub(viewport);
    let mut scroll_top = rebuild.scroll_y as usize;
    if rebuild.auto_scroll {
        scroll_top = max_start;
    } else {
        scroll_top = scroll_top.min(max_start);
    }
    rebuild.scroll_y = u16::try_from(scroll_top).unwrap_or(u16::MAX);

    let mut lines: Vec<Line> = job
        .output
        .iter()
        .map(|entry| match entry.stream {
            OutputStream::Stdout => {
                Line::from(vec![Span::raw(normalize_line(&entry.text, content_width))])
            }
            OutputStream::Stderr => Line::from(vec![Span::styled(
                normalize_line(&entry.text, content_width),
                Style::default().fg(Color::LightRed),
            )]),
        })
        .collect();

    if lines.len() < viewport {
        let pad = if content_width == 0 {
            String::new()
        } else {
            " ".repeat(content_width as usize)
        };
        lines.resize_with(viewport, || Line::from(pad.clone()));
    }

    let start_index = scroll_top.min(max_start);
    let visible: Vec<Line> = lines.into_iter().skip(start_index).take(viewport).collect();

    if let Some(number_area) = number_area {
        let separator = if gutter_width > line_digits as u16 {
            " "
        } else {
            ""
        };
        let blank_label = format!(
            "{:>width$}{separator}",
            "",
            width = line_digits,
            separator = separator
        );
        let mut number_lines: Vec<Line> = Vec::with_capacity(visible.len());
        for (offset, _) in visible.iter().enumerate() {
            let idx = start_index + offset;
            let label = if idx < job.output.len() {
                format!(
                    "{:>width$}{separator}",
                    idx + 1,
                    width = line_digits,
                    separator = separator
                )
            } else {
                blank_label.clone()
            };
            number_lines.push(Line::from(vec![Span::styled(
                label,
                Style::default().fg(Color::DarkGray),
            )]));
        }
        let gutter = Paragraph::new(number_lines);
        frame.render_widget(gutter, number_area);
    }

    let paragraph = Paragraph::new(visible).scroll((0, rebuild.scroll_x));

    frame.render_widget(paragraph, text_area);
}

fn draw_rebuild_sidebar(frame: &mut Frame, area: ratatui::prelude::Rect, rebuild: &RebuildState) {
    let total = rebuild.jobs.len();
    let active = rebuild.active_idx + 1;
    let mut lines: Vec<Line> = Vec::new();

    // Clearing keeps the sidebar from inheriting leftovers if the rebuild view resizes.
    frame.render_widget(Clear, area);

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

// TODO: i don't love that we have to strip chars to fix screen tear
//       but i couldn't find a way to get rid of them in the source output
fn normalize_line(text: &str, _content_width: u16) -> String {
    let segment = text.rsplit('\r').next().unwrap_or(text);
    let expanded = segment.replace('\t', "    ");
    expanded
}

fn count_digits(mut n: usize) -> usize {
    let mut digits = 1;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits
}

fn legend_lines() -> Vec<Line<'static>> {
    vec![
        Line::from(vec![styled_key("w", Color::Cyan), Span::raw(" Work queue")]),
        Line::from(vec![
            styled_key("↑/↓/←/→", Color::Yellow),
            Span::raw(" Scroll"),
        ]),
        Line::from(vec![
            styled_key("PgUp/PgDn", Color::Yellow),
            Span::raw(" Page scroll"),
        ]),
        Line::from(vec![
            styled_key("Home", Color::Yellow),
            Span::raw(" Goto top"),
        ]),
        Line::from(vec![
            styled_key("End", Color::Yellow),
            Span::raw(" Goto end"),
        ]),
        Line::from(vec![
            styled_key("esc", Color::Magenta),
            Span::raw(" Back to list"),
        ]),
        Line::from(vec![styled_key("q", Color::Red), Span::raw(" Quit")]),
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
        ViewMode::ByImage => {
            let containers = containers_for_image(app, &it.image);
            Row::new([
                Cell::from(checkbox),
                Cell::from(it.image.clone()),
                Cell::from(containers),
            ])
        }
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
                    ViewMode::ByImage => rows.push(Row::new([
                        Cell::from(""),
                        Cell::from(indented.clone()),
                        Cell::from(""),
                    ])),
                    ViewMode::ByFolderThenImage => {
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
            Span::raw("/"),
            styled_key("Esc", Color::Red),
            Span::raw(" quit"),
        ]),
        Line::from(vec![styled_key("v", Color::Cyan), Span::raw(" View")]),
    ]
}

fn containers_for_image(app: &App, image: &str) -> String {
    let mut containers: Vec<String> = app
        .all_items
        .iter()
        .filter_map(|item| {
            if item.image == image {
                item.container.clone()
            } else {
                None
            }
        })
        .collect();
    if containers.is_empty() {
        return "—".to_string();
    }
    containers.sort();
    containers.dedup();
    containers.join(", ")
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
