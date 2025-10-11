use crate::domain::DiscoveredImage;
use crate::tui::app::state::{App, Msg, SPINNER_FRAMES, Services, UiState, ViewMode};

pub fn handle_tick(app: &mut App) {
    app.spinner_idx = (app.spinner_idx + 1) % SPINNER_FRAMES.len();
}

pub fn handle_scan_results(
    app: &mut App,
    discovered: Vec<DiscoveredImage>,
    services: Option<&Services>,
) {
    app.all_items = discovered;
    app.rows = match app.view_mode {
        ViewMode::ByContainer => app.build_rows_for_container_view(),
        ViewMode::ByImage => app.build_rows_for_view_mode(ViewMode::ByImage),
        ViewMode::ByFolderThenImage => app.build_rows_for_folder_view(),
    };
    app.state = UiState::Ready;
    app.selected = 0;

    if app.auto_rebuild_all && !app.auto_rebuild_triggered {
        for row in &mut app.rows {
            if !row.is_dir {
                row.checked = true;
            }
        }
        app.auto_rebuild_triggered = true;

        if let Some(svc) = services {
            let _ = svc.tx.send(Msg::StartRebuild);
        }
    }
}

pub fn handle_details_ready(app: &mut App, row_idx: usize, details: Vec<String>) {
    if let Some(row) = app.rows.get_mut(row_idx) {
        row.details = details;
        row.expanded = true;
    }
}
