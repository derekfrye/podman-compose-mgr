use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use crate::tui::app::{App, ItemRow, ViewMode};

pub(crate) fn draw_table(frame: &mut Frame, area: Rect, app: &App) {
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
        ViewMode::ByDockerfile => (
            Row::new([
                Cell::from("Select"),
                Cell::from("Dockerfile"),
                Cell::from("Image"),
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
            vec![
                Constraint::Length(6),
                Constraint::Percentage(45),
                Constraint::Percentage(49),
            ],
        ),
    };

    let (rows, selected_visual_idx) = build_rows_with_expansion(app);

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().title("Images").borders(Borders::ALL))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

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
        ViewMode::ByDockerfile => {
            let dockerfile_name = it
                .dockerfile_extra
                .as_ref()
                .map_or_else(|| it.image.clone(), |extra| extra.dockerfile_name.clone());
            Row::new([
                Cell::from(checkbox),
                Cell::from(dockerfile_name),
                Cell::from(it.image.clone()),
            ])
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
            for line in &it.details {
                let indented = format!("  {line}");
                match app.view_mode {
                    ViewMode::ByContainer | ViewMode::ByImage | ViewMode::ByDockerfile => {
                        rows.push(Row::new([
                            Cell::from(""),
                            Cell::from(indented.clone()),
                            Cell::from(""),
                        ]));
                    }
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
