use crate::app::AppCore;
use crate::tui::app::state::{App, Msg, Services, ViewMode};

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
    let (image, source_dir, already_expanded) = match app.rows.get(app.selected) {
        Some(row) => (row.image.clone(), row.source_dir.clone(), row.expanded),
        None => return,
    };

    if already_expanded {
        return;
    }

    if let Some(row_mut) = app.rows.get_mut(app.selected) {
        row_mut.details = vec!["Loading details...".into()];
        row_mut.expanded = true;
    }

    if let Some(svc) = services {
        spawn_detail_fetch(svc, app.selected, image, source_dir);
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

fn spawn_detail_fetch(
    services: &Services,
    row_idx: usize,
    image: String,
    source_dir: std::path::PathBuf,
) {
    let tx = services.tx.clone();
    let core = services.core.clone();
    std::thread::spawn(move || {
        let details = compute_details_for(&core, &image, &source_dir);
        let _ = tx.send(Msg::DetailsReady {
            row: row_idx,
            details,
        });
    });
}

fn compute_details_for(core: &AppCore, image: &str, source_dir: &std::path::Path) -> Vec<String> {
    use crate::domain::ImageDetails;

    let mut lines = vec![format!("Compose dir: {}", source_dir.display())];
    match core.image_details(image, source_dir) {
        Ok(ImageDetails {
            created_time_ago,
            pulled_time_ago,
            has_dockerfile,
            has_makefile,
        }) => {
            if let Some(created) = created_time_ago {
                lines.push(format!("Created: {created}"));
            }
            if let Some(pulled) = pulled_time_ago {
                lines.push(format!("Pulled: {pulled}"));
            }
            if has_dockerfile {
                lines.push("Found Dockerfile".to_string());
            }
            if has_makefile {
                lines.push("Found Makefile".to_string());
            }
        }
        Err(err) => lines.push(format!("error: {err}")),
    }
    lines
}
