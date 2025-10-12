use super::rebuild_worker::spawn_rebuild_thread;
use crate::tui::app::state::{
    App, ModalState, Msg, OutputStream, RebuildJob, RebuildJobSpec, RebuildResult, RebuildState,
    RebuildStatus, Services, UiState,
};

pub fn handle_rebuild_message(app: &mut App, msg: Msg, services: Option<&Services>) {
    match msg {
        Msg::StartRebuild => handle_start_rebuild(app, services),
        Msg::RebuildSessionCreated { jobs } => handle_session_created(app, &jobs),
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
        handle_session_created(app, &specs);
        spawn_rebuild_thread(specs, svc);
    }
}

fn handle_session_created(app: &mut App, jobs: &[RebuildJobSpec]) {
    if jobs.is_empty() {
        return;
    }
    let materialized: Vec<RebuildJob> = jobs.iter().map(RebuildJob::from_spec).collect();
    app.state = UiState::Rebuilding;
    app.rebuild = Some(RebuildState::new(materialized));
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
    }
}

fn handle_job_output(app: &mut App, job_idx: usize, chunk: String, stream: OutputStream) {
    if let Some(rebuild) = app.rebuild.as_mut()
        && let Some(job) = rebuild.jobs.get_mut(job_idx)
    {
        let viewport = usize::from(rebuild.viewport_height.get().max(1));
        let bottom_threshold = job.output.len().saturating_sub(viewport);
        let was_at_bottom = (rebuild.scroll_y as usize) >= bottom_threshold;
        match stream {
            OutputStream::Stdout | OutputStream::Stderr => job.push_output(stream, chunk),
        }
        if rebuild.auto_scroll || was_at_bottom {
            rebuild.scroll_y = clamp_usize_to_u16(job.output.len().saturating_sub(viewport));
            rebuild.auto_scroll = true;
        }
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
        app.rebuild = None;
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
        let viewport = usize::from(rebuild.viewport_height.get().max(1));
        let max_scroll = clamp_usize_to_i32(job.output.len().saturating_sub(viewport));
        if max_scroll >= 0 {
            next = next.min(max_scroll);
        }
        rebuild.scroll_y = clamp_i32_to_u16(next);
    }
}

fn set_vertical_scroll(app: &mut App, value: u16) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        let viewport = usize::from(rebuild.viewport_height.get().max(1));
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
        let viewport = usize::from(rebuild.viewport_height.get().max(1));
        let bottom = clamp_usize_to_u16(job.output.len().saturating_sub(viewport));
        rebuild.scroll_y = bottom;
        rebuild.auto_scroll = true;
    }
}

fn adjust_horizontal_scroll(app: &mut App, delta: i32) {
    if let Some(rebuild) = app.rebuild.as_mut() {
        rebuild.auto_scroll = false;
        let current = i32::from(rebuild.scroll_x);
        let mut next = current + delta;
        if next < 0 {
            next = 0;
        }
        rebuild.scroll_x = clamp_i32_to_u16(next.min(5000));
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
