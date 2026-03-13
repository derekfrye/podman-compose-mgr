use crate::tui::app::state::{App, DockerfileRowExtra, MakefileRowExtra, Msg, Services, ViewMode};

use super::expansion_details::compute_details_for;

pub fn handle_expand_or_enter(app: &mut App, services: Option<&Services>) {
    if app.view_mode == ViewMode::ByFolderThenImage && handle_directory_enter(app) {
        return;
    }
    handle_row_expand(app, services);
}

pub fn handle_collapse_or_back(app: &mut App) {
    if app.view_mode == ViewMode::ByFolderThenImage {
        if collapse_selected_row(app) {
            return;
        }
        if let Some(previous) = app.current_path.pop() {
            app.rows = app.build_rows_for_folder_view();
            app.selected = app
                .rows
                .iter()
                .position(|row| row.is_dir && row.dir_name.as_deref() == Some(&previous))
                .unwrap_or(0);
        }
    } else if let Some(row) = app.rows.get_mut(app.selected) {
        row.expanded = false;
    }
}

fn handle_directory_enter(app: &mut App) -> bool {
    if let Some(row) = app.rows.get(app.selected)
        && row.is_dir
        && let Some(name) = &row.dir_name
    {
        app.current_path.push(name.clone());
        app.rows = app.build_rows_for_folder_view();
        app.selected = 0;
        return true;
    }
    false
}

fn handle_row_expand(app: &mut App, services: Option<&Services>) {
    let expansion = match app.rows.get(app.selected) {
        Some(row) => RowExpansionRequest {
            row_idx: app.selected,
            image: row.image.clone(),
            source_dir: row.source_dir.clone(),
            entry_path: row.entry_path.clone(),
            view_mode: app.view_mode,
            dockerfile_extra: row.dockerfile_extra.clone(),
            makefile_extra: row.makefile_extra.clone(),
            already_expanded: row.expanded,
        },
        None => return,
    };

    if expansion.already_expanded {
        return;
    }

    if let Some(row_mut) = app.rows.get_mut(app.selected) {
        row_mut.details = vec!["Loading details...".into()];
        row_mut.expanded = true;
    }

    if let Some(svc) = services {
        spawn_detail_fetch(svc, expansion);
    }
}

fn collapse_selected_row(app: &mut App) -> bool {
    if let Some(row) = app.rows.get_mut(app.selected)
        && !row.is_dir
        && row.expanded
    {
        row.expanded = false;
        return true;
    }
    false
}

#[derive(Clone)]
struct RowExpansionRequest {
    row_idx: usize,
    image: String,
    source_dir: std::path::PathBuf,
    entry_path: Option<std::path::PathBuf>,
    view_mode: ViewMode,
    dockerfile_extra: Option<DockerfileRowExtra>,
    makefile_extra: Option<MakefileRowExtra>,
    already_expanded: bool,
}

fn spawn_detail_fetch(services: &Services, request: RowExpansionRequest) {
    let tx = services.tx.clone();
    let core = services.core.clone();
    std::thread::spawn(move || {
        let entry_path_ref = request.entry_path.as_deref();
        let details = compute_details_for(
            &core,
            &request.image,
            &request.source_dir,
            entry_path_ref,
            request.view_mode,
            request.dockerfile_extra.as_ref(),
            request.makefile_extra.as_ref(),
        );
        let _ = tx.send(Msg::DetailsReady {
            row: request.row_idx,
            details,
        });
    });
}
