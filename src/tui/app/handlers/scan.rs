use crate::domain::DiscoveredImage;
use crate::tui::app::state::{App, SPINNER_FRAMES, UiState, ViewMode};

pub fn handle_tick(app: &mut App) {
    app.spinner_idx = (app.spinner_idx + 1) % SPINNER_FRAMES.len();
}

pub fn handle_scan_results(app: &mut App, discovered: Vec<DiscoveredImage>) {
    app.all_items = discovered;
    app.rows = match app.view_mode {
        ViewMode::ByContainer => app.build_rows_for_container_view(),
        ViewMode::ByImage => app.build_rows_for_view_mode(ViewMode::ByImage),
        ViewMode::ByFolderThenImage => app.build_rows_for_folder_view(),
    };
    app.state = UiState::Ready;
    app.selected = 0;
}

pub fn handle_details_ready(app: &mut App, row_idx: usize, details: Vec<String>) {
    if let Some(row) = app.rows.get_mut(row_idx) {
        row.details = details;
        row.expanded = true;
    }
}
