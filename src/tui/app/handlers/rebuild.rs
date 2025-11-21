use super::rebuild_worker::spawn_rebuild_thread;
use crate::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT;
use crate::tui::app::search::{SearchDirection, SearchState};
use crate::tui::app::state::{
    App, ModalState, Msg, OutputStream, RebuildJob, RebuildJobSpec, RebuildResult, RebuildState,
    RebuildStatus, Services, UiState,
};
use chrono::Local;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path};
use unicode_width::UnicodeWidthChar;

pub fn handle_rebuild_message(app: &mut App, msg: Msg, services: Option<&Services>) {
    match msg {
        Msg::StartRebuild => handle_start_rebuild(app, services),
        Msg::RebuildSessionCreated { jobs } => handle_session_created(app, &jobs, services),
        Msg::RebuildJobStarted { job_idx } => handle_job_started(app, job_idx),
        Msg::RebuildJobOutput {
            job_idx,
            chunk,
            stream,
        } => handle_job_output(app, job_idx, chunk, stream),
        Msg::RebuildJobFinished { job_idx, result } => handle_job_finished(app, job_idx, result),
        Msg::RebuildAdvance => handle_rebuild_advance(app, services),
        Msg::RebuildAborted(reason) => handle_rebuild_aborted(app, reason),
        Msg::RebuildAllDone => handle_rebuild_complete(app),
        Msg::OpenWorkQueue => handle_open_work_queue(app),
        Msg::CloseModal => handle_close_modal(app),
        Msg::WorkQueueUp => handle_work_queue_up(app),
        Msg::WorkQueueDown => handle_work_queue_down(app),
        Msg::WorkQueueSelect => handle_work_queue_select(app),
        Msg::ToggleCheckAll => handle_toggle_check_all(app),
        Msg::ScrollOutputUp
        | Msg::ScrollOutputDown
        | Msg::ScrollOutputPageUp
        | Msg::ScrollOutputPageDown
        | Msg::ScrollOutputTop
        | Msg::ScrollOutputBottom
        | Msg::ScrollOutputLeft
        | Msg::ScrollOutputRight => handle_scroll_message(app, &msg),
        Msg::OpenExportLog => handle_open_export_log(app),
        Msg::ExportInput(ch) => handle_export_input(app, ch),
        Msg::ExportBackspace => handle_export_backspace(app),
        Msg::ExportCancel => handle_export_cancel(app),
        Msg::ExportSubmit => handle_export_submit(app, services),
        Msg::StartSearchForward => handle_search_start(app, SearchDirection::Forward),
        Msg::StartSearchBackward => handle_search_start(app, SearchDirection::Backward),
        Msg::SearchInput(ch) => handle_search_input(app, ch),
        Msg::SearchBackspace => handle_search_backspace(app),
        Msg::SearchSubmit => handle_search_submit(app),
        Msg::SearchCancel => handle_search_cancel(app),
        Msg::SearchNext => handle_search_next(app),
        Msg::SearchPrev => handle_search_prev(app),
        Msg::ShowRebuild => handle_show_rebuild(app),
        Msg::ExitRebuild => handle_exit_rebuild(app),
        _ => {}
    }
}

fn handle_start_rebuild(app: &mut App, services: Option<&Services>) {
    if app.state != UiState::Ready || services.is_none() {
        return;
    }

    let specs = collect_selected_specs(app);
    if specs.is_empty() {
        return;
    }

    // Clear checkboxes so the selection is obvious when returning
    for row in &mut app.rows {
        if row.checked {
            row.checked = false;
        }
    }

    if let Some(svc) = services {
        let start_idx = app
            .rebuild
            .as_ref()
            .map(|state| state.jobs.len())
            .unwrap_or(0);
        handle_session_created(app, &specs, services);
        spawn_rebuild_thread(specs, svc, start_idx);
    }
}

fn handle_session_created(app: &mut App, jobs: &[RebuildJobSpec], services: Option<&Services>) {
    if jobs.is_empty() {
        return;
    }
    let limit = services
        .map(|svc| svc.args.rebuild_view_line_buffer_max)
        .unwrap_or(REBUILD_VIEW_LINE_BUFFER_DEFAULT);
    let materialized: Vec<RebuildJob> = jobs.iter().map(RebuildJob::from_spec).collect();

    match app.rebuild.as_mut() {
        Some(rebuild) => {
            rebuild.jobs.extend(materialized);
            rebuild.finished = false;
            rebuild.output_limit = limit;
            if rebuild.work_queue_selected >= rebuild.jobs.len() {
                rebuild.work_queue_selected = rebuild.jobs.len().saturating_sub(1);
            }
        }
        None => {
            app.rebuild = Some(RebuildState::new(materialized, limit));
        }
    }
    app.state = UiState::Rebuilding;
}

fn handle_job_started(app: &mut App, job_idx: usize) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(job_idx)
    {
        job.status = RebuildStatus::Running;
        rebuild.active_idx = job_idx;
        rebuild.work_queue_selected = job_idx;
        rebuild.scroll_y = 0;
        rebuild.scroll_x = 0;
        rebuild.auto_scroll = true;
        refresh_search_for_active_job(rebuild);
    }
}

fn handle_job_output(app: &mut App, job_idx: usize, chunk: String, stream: OutputStream) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(job_idx)
    {
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let bottom_threshold = job.output.len().saturating_sub(viewport);
        let was_at_bottom = (rebuild.scroll_y as usize) >= bottom_threshold;
        match stream {
            OutputStream::Stdout | OutputStream::Stderr => {
                job.push_output(stream, chunk, rebuild.output_limit)
            }
        }
        if rebuild.auto_scroll || was_at_bottom {
            rebuild.scroll_y = clamp_usize_to_u16(job.output.len().saturating_sub(viewport));
            rebuild.auto_scroll = true;
        }
        refresh_search_for_active_job(rebuild);
    }
}

fn handle_job_finished(app: &mut App, job_idx: usize, result: RebuildResult) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(job_idx)
    {
        match result {
            RebuildResult::Success => job.status = RebuildStatus::Succeeded,
            RebuildResult::Failure(err) => {
                job.status = RebuildStatus::Failed;
                job.error = Some(err);
            }
            RebuildResult::Cancelled => job.status = RebuildStatus::Failed,
        }
        refresh_search_for_active_job(rebuild);
    }
}

fn handle_rebuild_advance(_app: &mut App, _services: Option<&crate::tui::app::state::Services>) {}

fn handle_rebuild_aborted(app: &mut App, reason: String) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(rebuild.active_idx)
    {
        job.status = RebuildStatus::Failed;
        job.error = Some(reason);
    }
    app.state = super::super::state::UiState::Ready;
    app.rebuild = None;
}

fn handle_rebuild_complete(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        rebuild.finished = true;
        rebuild.auto_scroll = true;
    }
}

fn handle_exit_rebuild(app: &mut App) {
    if matches!(app.state, UiState::Rebuilding) {
        app.state = UiState::Ready;
        app.modal = None;
    }
}

fn handle_show_rebuild(app: &mut App) {
    if app.rebuild.is_some() {
        app.state = UiState::Rebuilding;
        app.modal = None;
    }
}

fn handle_open_work_queue(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_ref() {
        app.modal = Some(ModalState::WorkQueue {
            selected_idx: rebuild
                .work_queue_selected
                .min(rebuild.jobs.len().saturating_sub(1)),
        });
    }
}

fn handle_close_modal(app: &mut App) {
    app.modal = None;
}

fn handle_work_queue_up(app: &mut App) {
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

fn handle_work_queue_down(app: &mut App) {
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

fn handle_work_queue_select(app: &mut App) {
    if let Some(ModalState::WorkQueue { selected_idx }) = app.modal
        && let Some(rebuild) = app.rebuild.as_mut()
        && selected_idx < rebuild.jobs.len()
    {
        rebuild.active_idx = selected_idx;
        rebuild.work_queue_selected = selected_idx;
        rebuild.scroll_y = 0;
        rebuild.scroll_x = 0;
        rebuild.auto_scroll = true;
        refresh_search_for_active_job(rebuild);
    }
    app.modal = None;
}

fn handle_toggle_check_all(app: &mut App) {
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

fn collect_selected_specs(app: &App) -> Vec<RebuildJobSpec> {
    app.rows
        .iter()
        .filter(|row| row.checked && !row.is_dir)
        .filter_map(|row| {
            let entry_path = row.entry_path.as_ref()?;
            Some(RebuildJobSpec {
                image: row.image.clone(),
                container: row.container.clone(),
                entry_path: entry_path.clone(),
                source_dir: row.source_dir.clone(),
            })
        })
        .collect()
}

fn handle_scroll_message(app: &mut App, msg: &Msg) {
    match msg {
        Msg::ScrollOutputUp => adjust_vertical_scroll(app, -1),
        Msg::ScrollOutputDown => adjust_vertical_scroll(app, 1),
        Msg::ScrollOutputPageUp => adjust_vertical_scroll(app, -12),
        Msg::ScrollOutputPageDown => adjust_vertical_scroll(app, 12),
        Msg::ScrollOutputTop => set_vertical_scroll(app, 0),
        Msg::ScrollOutputBottom => set_vertical_to_bottom(app),
        Msg::ScrollOutputLeft => adjust_horizontal_scroll(app, -4),
        Msg::ScrollOutputRight => adjust_horizontal_scroll(app, 4),
        _ => {}
    }
}

fn handle_search_start(app: &mut App, direction: SearchDirection) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let search = rebuild
        .search
        .get_or_insert_with(|| SearchState::new(direction));
    search.set_direction(direction);
    search.editing = true;
    search.error = None;
    if search.has_query() {
        if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
            let baseline = usize::from(rebuild.scroll_y);
            search.recompute_matches(job, baseline);
        }
    } else {
        search.clear_results();
    }
}

fn handle_search_input(app: &mut App, ch: char) {
    if app.state != UiState::Rebuilding || ch.is_control() {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let Some(search) = rebuild.search.as_mut() else {
        return;
    };
    if !search.editing {
        return;
    }
    search.push_char(ch);
    search.error = None;
    if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
        let baseline = usize::from(rebuild.scroll_y);
        search.recompute_matches(job, baseline);
    }
}

fn handle_search_backspace(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let Some(search) = rebuild.search.as_mut() else {
        return;
    };
    if !search.editing {
        return;
    }
    search.pop_char();
    search.error = None;
    if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
        let baseline = usize::from(rebuild.scroll_y);
        search.recompute_matches(job, baseline);
    } else {
        search.clear_results();
    }
}

fn handle_search_submit(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut focus_result = false;
    let mut drop_search = false;

    if let Some(search) = rebuild.search.as_mut() {
        if !search.editing {
            focus_result = search.active.is_some();
        } else if search.query.is_empty() {
            drop_search = true;
        } else if let Some(job) = rebuild.jobs.get(rebuild.active_idx) {
            let baseline = usize::from(rebuild.scroll_y);
            search.recompute_matches(job, baseline);
            if search.error.is_none() {
                search.editing = false;
                if search.active.is_none() && !search.matches.is_empty() {
                    search.active = Some(0);
                }
                focus_result = true;
            }
        }
    }

    if drop_search {
        rebuild.search = None;
    } else if focus_result {
        focus_on_active_search_match(rebuild);
    }
}

fn handle_search_cancel(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut remove = false;
    if let Some(search) = rebuild.search.as_mut() {
        if search.editing {
            if search.has_query() && search.regex.is_some() {
                search.editing = false;
                search.error = None;
            } else {
                remove = true;
            }
        } else {
            remove = true;
        }
    }
    if remove {
        rebuild.search = None;
    }
}

fn handle_search_next(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut focus = false;
    if let Some(search) = rebuild.search.as_mut() {
        if search.editing || search.matches.is_empty() {
            return;
        }
        search.set_direction(SearchDirection::Forward);
        search.advance_next();
        focus = true;
    }
    if focus {
        focus_on_active_search_match(rebuild);
    }
}

fn handle_search_prev(app: &mut App) {
    if app.state != UiState::Rebuilding {
        return;
    }
    let Some(rebuild) = app.rebuild.as_mut() else {
        return;
    };
    let mut focus = false;
    if let Some(search) = rebuild.search.as_mut() {
        if search.editing || search.matches.is_empty() {
            return;
        }
        search.set_direction(SearchDirection::Backward);
        search.advance_prev();
        focus = true;
    }
    if focus {
        focus_on_active_search_match(rebuild);
    }
}

fn refresh_search_for_active_job(rebuild: &mut RebuildState) {
    if let Some(search) = rebuild.search.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        if search.has_query() {
            let baseline = usize::from(rebuild.scroll_y);
            search.recompute_matches(job, baseline);
        } else {
            search.clear_results();
        }
    }
}

fn focus_on_active_search_match(rebuild: &mut RebuildState) {
    let Some(search) = rebuild.search.as_ref() else {
        return;
    };
    let Some(active_idx) = search.active else {
        return;
    };
    let Some(hit) = search.matches.get(active_idx) else {
        return;
    };
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        return;
    };

    let viewport = usize::from(rebuild.viewport_height.max(1));
    let max_start = job.output.len().saturating_sub(viewport);
    let target = hit.line.saturating_sub(viewport / 2);
    rebuild.scroll_y = clamp_usize_to_u16(target.min(max_start));
    rebuild.auto_scroll = false;
}

fn adjust_vertical_scroll(app: &mut App, delta: i32) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        rebuild.auto_scroll = false;
        let current = i32::from(rebuild.scroll_y);
        let mut next = current + delta;
        if next < 0 {
            next = 0;
        }
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let max_scroll = clamp_usize_to_i32(job.output.len().saturating_sub(viewport));
        if max_scroll >= 0 {
            next = next.min(max_scroll);
        }
        rebuild.scroll_y = clamp_i32_to_u16(next);
    }
}

fn set_vertical_scroll(app: &mut App, value: u16) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let max_scroll = clamp_usize_to_u16(
            rebuild
                .jobs
                .get(rebuild.active_idx)
                .map(|job| job.output.len().saturating_sub(viewport))
                .unwrap_or(0),
        );
        rebuild.scroll_y = value.min(max_scroll);
        rebuild.auto_scroll = false;
    }
}

fn set_vertical_to_bottom(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let bottom = clamp_usize_to_u16(job.output.len().saturating_sub(viewport));
        rebuild.scroll_y = bottom;
        rebuild.auto_scroll = true;
    }
}

fn adjust_horizontal_scroll(app: &mut App, delta: i32) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get(rebuild.active_idx)
    {
        let viewport = usize::from(rebuild.viewport_width.max(1));
        let max_line_width = job
            .output
            .iter()
            .map(|entry| line_display_width(&entry.text))
            .max()
            .unwrap_or(0);
        let (max_offset, step) = if max_line_width == 0 {
            (0usize, 0usize)
        } else if max_line_width > viewport {
            (
                max_line_width.saturating_sub(viewport),
                (viewport * 2 / 3).max(1),
            )
        } else {
            let target = max_line_width.saturating_sub(1).min(4);
            (target, target.max(1))
        };

        if max_offset == 0 {
            rebuild.scroll_x = 0;
            rebuild.auto_scroll = false;
            return;
        }

        let current = usize::from(rebuild.scroll_x);
        let mut next = if delta >= 0 {
            current.saturating_add(step)
        } else {
            current.saturating_sub(step)
        };
        if delta >= 0 {
            next = next.min(max_offset);
        }
        rebuild.scroll_x = clamp_usize_to_u16(next);
        rebuild.auto_scroll = false;
    }
}

fn clamp_usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn clamp_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn clamp_i32_to_u16(value: i32) -> u16 {
    let non_negative = value.max(0);
    u16::try_from(non_negative).unwrap_or(u16::MAX)
}

fn line_display_width(text: &str) -> usize {
    let segment = text.rsplit('\r').next().unwrap_or(text);
    segment
        .replace('\t', "    ")
        .chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
        .sum()
}

fn handle_open_export_log(app: &mut App) {
    if !matches!(app.state, UiState::Rebuilding) {
        return;
    }
    let Some(rebuild) = app.rebuild.as_ref() else {
        return;
    };
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        return;
    };

    let default_name = default_export_filename(job);
    app.modal = Some(ModalState::ExportLog {
        input: default_name,
        error: None,
    });
}

fn handle_export_input(app: &mut App, ch: char) {
    if let Some(ModalState::ExportLog { input, error }) = app.modal.as_mut() {
        input.push(ch);
        *error = None;
    }
}

fn handle_export_backspace(app: &mut App) {
    if let Some(ModalState::ExportLog { input, error }) = app.modal.as_mut() {
        input.pop();
        *error = None;
    }
}

fn handle_export_cancel(app: &mut App) {
    if matches!(app.modal, Some(ModalState::ExportLog { .. })) {
        app.modal = None;
    }
}

fn handle_export_submit(app: &mut App, services: Option<&Services>) {
    let Some(services) = services else {
        return;
    };
    let Some(rebuild) = app.rebuild.as_ref() else {
        return;
    };
    let Some(job) = rebuild.jobs.get(rebuild.active_idx) else {
        return;
    };
    let filename = match &app.modal {
        Some(ModalState::ExportLog { input, .. }) => input.clone(),
        _ => return,
    };
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        set_export_error(app, Some("File name cannot be empty".to_string()));
        return;
    }

    let candidate = Path::new(trimmed);
    if candidate.is_absolute() {
        set_export_error(
            app,
            Some("Provide a relative filename (absolute paths are not allowed).".to_string()),
        );
        return;
    }

    if contains_forbidden_path_segments(candidate) {
        set_export_error(
            app,
            Some("File name cannot traverse parent directories.".to_string()),
        );
        return;
    }

    let mut destination = services.working_dir.clone();
    destination.push(candidate);

    let lines: Vec<String> = job.output.iter().map(|entry| entry.text.clone()).collect();
    let active_idx = rebuild.active_idx;

    match write_lines(&destination, &lines) {
        Ok(()) => {
            if let Some(rebuild_mut) = app.rebuild.as_mut()
                && let Some(job_mut) = rebuild_mut.jobs.get_mut(active_idx)
            {
                job_mut.push_output(
                    OutputStream::Stdout,
                    format!("Exported rebuild log to {}", destination.display()),
                    rebuild_mut.output_limit,
                );
                rebuild_mut.auto_scroll = true;
            }
            app.modal = None;
        }
        Err(err) => {
            set_export_error(app, Some(err));
        }
    }
}

fn set_export_error(app: &mut App, message: Option<String>) {
    if let Some(ModalState::ExportLog { error, .. }) = app.modal.as_mut() {
        *error = message;
    }
}

fn write_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            return Err(format!("Failed creating directory: {err}"));
        }
    }

    let mut file = File::create(path).map_err(|err| format!("Could not create file: {err}"))?;
    for line in lines {
        writeln!(file, "{line}").map_err(|err| format!("Failed writing file: {err}"))?;
    }
    file.flush()
        .map_err(|err| format!("Failed to flush file: {err}"))
}

fn default_export_filename(job: &RebuildJob) -> String {
    let image = job.image.as_str();
    let (name_raw, tag_raw) = split_image_name_and_tag(image);
    let name = sanitize_filename_component(name_raw).unwrap_or_else(|| "image".to_string());
    let tag = sanitize_filename_component(tag_raw).unwrap_or_else(|| "tag".to_string());
    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    format!("{name}-{tag}-{timestamp}.log")
}

fn split_image_name_and_tag(image: &str) -> (&str, &str) {
    if let Some(at_pos) = image.rfind('@') {
        let name = &image[..at_pos];
        let digest = &image[at_pos + 1..];
        return (name, digest);
    }

    let last_slash = image.rfind('/');
    if let Some(colon_pos) = image.rfind(':') {
        if last_slash.map_or(true, |slash_pos| slash_pos < colon_pos) {
            let name = &image[..colon_pos];
            let tag = &image[colon_pos + 1..];
            return (name, tag);
        }
    }

    (image, "latest")
}

fn sanitize_filename_component(input: &str) -> Option<String> {
    let filtered: String = input
        .chars()
        .filter(|ch| !matches!(ch, ':' | '/' | '\\' | '?' | '*' | '"' | '<' | '>' | ' '))
        .collect();
    let trimmed = filtered.trim_matches('.');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn contains_forbidden_path_segments(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_image_with_tag() {
        let (name, tag) = split_image_name_and_tag("nginx:latest");
        assert_eq!(name, "nginx");
        assert_eq!(tag, "latest");
    }

    #[test]
    fn split_image_with_registry_port() {
        let (name, tag) = split_image_name_and_tag("registry.local:5000/foo/bar:dev");
        assert_eq!(name, "registry.local:5000/foo/bar");
        assert_eq!(tag, "dev");
    }

    #[test]
    fn split_image_with_digest() {
        let (name, tag) = split_image_name_and_tag("registry.example.com/foo@sha256:abcdef123456");
        assert_eq!(name, "registry.example.com/foo");
        assert_eq!(tag, "sha256:abcdef123456");
    }

    #[test]
    fn sanitize_component_removes_chars() {
        let sanitized = sanitize_filename_component("foo/bar:tag name").unwrap();
        assert_eq!(sanitized, "foobartagname");
    }

    #[test]
    fn sanitize_component_returns_none_when_empty() {
        assert!(sanitize_filename_component("   ").is_none());
        assert!(sanitize_filename_component("::").is_none());
    }

    #[test]
    fn contains_forbidden_segments_detects_parent_dir() {
        assert!(contains_forbidden_path_segments(Path::new("../foo")));
        assert!(!contains_forbidden_path_segments(Path::new("foo/bar.log")));
    }
}
