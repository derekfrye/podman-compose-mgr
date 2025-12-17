use crate::tui::app::state::{App, ModalState, UiState};

pub(super) fn handle_open_work_queue(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_ref() {
        app.modal = Some(ModalState::WorkQueue {
            selected_idx: rebuild
                .work_queue_selected
                .min(rebuild.jobs.len().saturating_sub(1)),
        });
    }
}

pub(super) fn handle_close_modal(app: &mut App) {
    app.modal = None;
}

pub(super) fn handle_work_queue_up(app: &mut App) {
    if let Some(ModalState::WorkQueue { selected_idx }) = app.modal.as_mut()
        && *selected_idx > 0
    {
        *selected_idx -= 1;
        if let Some(rebuild) = app.rebuild.as_mut() {
            rebuild.work_queue_selected = *selected_idx;
            rebuild.auto_scroll = true;
        }
    }
}

pub(super) fn handle_work_queue_down(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_ref()
        && let Some(ModalState::WorkQueue { selected_idx }) = app.modal.as_mut()
        && *selected_idx + 1 < rebuild.jobs.len()
    {
        *selected_idx += 1;
        if let Some(rebuild) = app.rebuild.as_mut() {
            rebuild.work_queue_selected = *selected_idx;
            rebuild.auto_scroll = true;
        }
    }
}

pub(super) fn handle_work_queue_select(app: &mut App) {
    if let Some(ModalState::WorkQueue { selected_idx }) = app.modal
        && let Some(rebuild) = app.rebuild.as_mut()
        && selected_idx < rebuild.jobs.len()
    {
        rebuild.active_idx = selected_idx;
        rebuild.work_queue_selected = selected_idx;
        rebuild.scroll_y = 0;
        rebuild.scroll_x = 0;
        rebuild.auto_scroll = true;
        super::refresh_search_for_active_job(rebuild);
    }
    app.modal = None;
}

pub(super) fn handle_toggle_check_all(app: &mut App) {
    if app.state != UiState::Ready {
        return;
    }

    let should_check = app
        .rows
        .iter()
        .filter(|row| !row.is_dir)
        .any(|row| !row.checked);

    for row in &mut app.rows {
        if !row.is_dir {
            row.checked = should_check;
        }
    }
}
