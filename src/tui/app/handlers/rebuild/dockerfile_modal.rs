use crate::tui::app::state::{
    App, DockerfileNameEntry, ModalState, RebuildJobSpec, Services, ViewMode,
};

pub(super) fn open_dockerfile_modal(app: &mut App) -> bool {
    if app.view_mode != ViewMode::ByDockerfile {
        return false;
    }

    let mut entries: Vec<DockerfileNameEntry> = Vec::new();
    for row in &app.rows {
        if !row.checked || row.is_dir {
            continue;
        }
        let Some(entry_path) = row.entry_path.as_ref() else {
            continue;
        };
        let dockerfile_name = row
            .dockerfile_extra
            .as_ref()
            .map(|extra| extra.dockerfile_name.clone())
            .or_else(|| {
                entry_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "Dockerfile".to_string());
        entries.push(DockerfileNameEntry {
            dockerfile_path: entry_path.clone(),
            source_dir: row.source_dir.clone(),
            dockerfile_name,
            image_name: row.image.clone(),
            cursor: row.image.len(),
        });
    }

    if entries.is_empty() {
        return false;
    }

    app.modal = Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx: 0,
        error: None,
    });
    true
}

pub(super) fn handle_dockerfile_modal_move(app: &mut App, delta: i32) {
    let Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx,
        ..
    }) = app.modal.as_mut()
    else {
        return;
    };
    if entries.is_empty() {
        return;
    }
    let len = i32::try_from(entries.len()).unwrap_or(i32::MAX);
    let current = i32::try_from(*selected_idx).unwrap_or(0);
    let next = (current + delta).clamp(0, len.saturating_sub(1));
    if let Ok(next_idx) = usize::try_from(next) {
        *selected_idx = next_idx;
        if let Some(entry) = entries.get_mut(*selected_idx)
            && entry.cursor > entry.image_name.len()
        {
            entry.cursor = entry.image_name.len();
        }
    }
}

pub(super) fn handle_dockerfile_modal_input(app: &mut App, ch: char) {
    let Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx,
        error,
    }) = app.modal.as_mut()
    else {
        return;
    };
    if let Some(entry) = entries.get_mut(*selected_idx) {
        let pos = entry.cursor.min(entry.image_name.len());
        entry.image_name.insert(pos, ch);
        entry.cursor = pos.saturating_add(1);
    }
    *error = None;
}

pub(super) fn handle_dockerfile_modal_backspace(app: &mut App) {
    let Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx,
        error,
    }) = app.modal.as_mut()
    else {
        return;
    };
    if let Some(entry) = entries.get_mut(*selected_idx) {
        let pos = entry.cursor.min(entry.image_name.len());
        if pos > 0 {
            let remove_at = pos - 1;
            entry.image_name.remove(remove_at);
            entry.cursor = remove_at;
        }
    }
    *error = None;
}

pub(super) fn handle_dockerfile_modal_left(app: &mut App) {
    let Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx,
        ..
    }) = app.modal.as_mut()
    else {
        return;
    };
    if let Some(entry) = entries.get_mut(*selected_idx) {
        entry.cursor = entry.cursor.saturating_sub(1);
    }
}

pub(super) fn handle_dockerfile_modal_right(app: &mut App) {
    let Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx,
        ..
    }) = app.modal.as_mut()
    else {
        return;
    };
    if let Some(entry) = entries.get_mut(*selected_idx) {
        let len = entry.image_name.len();
        entry.cursor = (entry.cursor + 1).min(len);
    }
}

pub(super) fn handle_dockerfile_modal_accept(app: &mut App, services: Option<&Services>) {
    let Some(ModalState::DockerfileNameEdit {
        entries,
        selected_idx: _selected_idx,
        error: _error,
    }) = app.modal.take()
    else {
        return;
    };

    let invalid = entries
        .iter()
        .position(|entry| is_invalid_image_name(&entry.image_name));
    if let Some(idx) = invalid {
        app.modal = Some(ModalState::DockerfileNameEdit {
            entries,
            selected_idx: idx,
            error: Some("Set image names before rebuilding".to_string()),
        });
        return;
    }

    let specs: Vec<RebuildJobSpec> = entries
        .into_iter()
        .map(|entry| RebuildJobSpec {
            image: entry.image_name.trim().to_string(),
            container: None,
            entry_path: entry.dockerfile_path,
            source_dir: entry.source_dir,
            make_target: None,
        })
        .collect();

    clear_checked_rows(app);
    super::queue_rebuild_jobs(app, services, specs);
}

pub(super) fn handle_dockerfile_modal_cancel(app: &mut App) {
    if matches!(app.modal, Some(ModalState::DockerfileNameEdit { .. })) {
        app.modal = None;
    }
}

fn is_invalid_image_name(image: &str) -> bool {
    let trimmed = image.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown")
}

fn clear_checked_rows(app: &mut App) {
    for row in &mut app.rows {
        if row.checked {
            row.checked = false;
        }
    }
}
