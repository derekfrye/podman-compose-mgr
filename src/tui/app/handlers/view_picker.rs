use crate::tui::app::state::{App, ModalState, ViewMode};

pub fn handle_open_view_picker(app: &mut App) {
    let default_idx = match app.view_mode {
        ViewMode::ByContainer => 0,
        ViewMode::ByImage => 1,
        ViewMode::ByFolderThenImage => 2,
        ViewMode::ByDockerfile => 3,
        ViewMode::ByMakefile => 4,
    };
    app.modal = Some(ModalState::ViewPicker {
        selected_idx: default_idx,
    });
}

pub fn handle_view_picker_up(app: &mut App) {
    if let Some(ModalState::ViewPicker { selected_idx }) = &mut app.modal
        && *selected_idx > 0
    {
        *selected_idx -= 1;
    }
}

pub fn handle_view_picker_down(app: &mut App) {
    if let Some(ModalState::ViewPicker { selected_idx }) = &mut app.modal
        && *selected_idx < 4
    {
        *selected_idx += 1;
    }
}

pub fn handle_view_picker_accept(app: &mut App) {
    if let Some(ModalState::ViewPicker { selected_idx }) = &mut app.modal {
        app.view_mode = match *selected_idx {
            1 => ViewMode::ByImage,
            2 => ViewMode::ByFolderThenImage,
            3 => ViewMode::ByDockerfile,
            4 => ViewMode::ByMakefile,
            _ => ViewMode::ByContainer,
        };
        app.rebuild_rows_for_view();
        app.modal = None;
    }
}

pub fn handle_view_picker_select(app: &mut App, mode: ViewMode) {
    if matches!(app.modal, Some(ModalState::ViewPicker { .. })) {
        app.view_mode = mode;
        app.rebuild_rows_for_view();
        app.modal = None;
    }
}

pub fn handle_view_picker_cancel(app: &mut App) {
    app.modal = None;
}
