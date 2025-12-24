use super::default_line_buffer_limit;
use super::dockerfile_modal::open_dockerfile_modal;
use super::queue_rebuild_jobs;
use super::refresh_search_for_active_job;
use super::scroll::clamp_usize_to_u16;
use crate::tui::app::state::{
    App, OutputStream, RebuildJob, RebuildJobSpec, RebuildResult, RebuildState, RebuildStatus,
    Services, UiState, ViewMode,
};

pub(super) fn handle_start_rebuild(app: &mut App, services: Option<&Services>) {
    if app.state != UiState::Ready || services.is_none() {
        return;
    }

    if app.view_mode == ViewMode::ByDockerfile && open_dockerfile_modal(app) {
        return;
    }

    let specs = collect_selected_specs(app);
    if specs.is_empty() {
        return;
    }

    clear_checked_rows(app);

    queue_rebuild_jobs(app, services, specs);
}

pub(super) fn handle_session_created(
    app: &mut App,
    jobs: &[RebuildJobSpec],
    services: Option<&Services>,
) {
    if jobs.is_empty() {
        return;
    }
    let limit = default_line_buffer_limit(services);
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

pub(super) fn handle_job_started(app: &mut App, job_idx: usize) {
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

pub(super) fn handle_job_output(
    app: &mut App,
    job_idx: usize,
    chunk: String,
    stream: OutputStream,
) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(job_idx)
    {
        let viewport = usize::from(rebuild.viewport_height.max(1));
        let bottom_threshold = job.output.len().saturating_sub(viewport);
        let was_at_bottom = (rebuild.scroll_y as usize) >= bottom_threshold;
        match stream {
            super::super::super::state::OutputStream::Stdout
            | super::super::super::state::OutputStream::Stderr => {
                job.push_output(stream, chunk, rebuild.output_limit);
            }
        }
        if rebuild.auto_scroll || was_at_bottom {
            rebuild.scroll_y = clamp_usize_to_u16(job.output.len().saturating_sub(viewport));
            rebuild.auto_scroll = true;
        }
        refresh_search_for_active_job(rebuild);
    }
}

pub(super) fn handle_job_finished(app: &mut App, job_idx: usize, result: RebuildResult) {
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

pub(super) fn handle_rebuild_advance(_app: &mut App, _services: Option<&Services>) {}

pub(super) fn handle_rebuild_aborted(app: &mut App, reason: String) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(rebuild.active_idx)
    {
        job.status = RebuildStatus::Failed;
        job.error = Some(reason);
    }
    app.state = UiState::Ready;
    app.rebuild = None;
}

pub(super) fn handle_rebuild_complete(app: &mut App) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        rebuild.finished = true;
        rebuild.auto_scroll = true;
    }
}

pub(super) fn handle_exit_rebuild(app: &mut App) {
    if matches!(app.state, UiState::Rebuilding) {
        app.state = UiState::Ready;
        app.modal = None;
    }
}

pub(super) fn handle_show_rebuild(app: &mut App) {
    if app.rebuild.is_some() {
        app.state = UiState::Rebuilding;
        app.modal = None;
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
                make_target: row
                    .makefile_extra
                    .as_ref()
                    .and_then(|extra| extra.make_target.clone()),
            })
        })
        .collect()
}

fn clear_checked_rows(app: &mut App) {
    for row in &mut app.rows {
        if row.checked {
            row.checked = false;
        }
    }
}
