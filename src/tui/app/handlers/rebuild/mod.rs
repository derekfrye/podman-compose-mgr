mod dockerfile_modal;
mod export_log;
mod scroll;
mod search;
mod session;
mod work_queue;

use super::rebuild_worker::spawn_rebuild_thread;
use crate::args::types::REBUILD_VIEW_LINE_BUFFER_DEFAULT;
use crate::tui::app::search::SearchDirection;
use crate::tui::app::state::{App, Msg, RebuildJobSpec, RebuildState, Services};

pub fn handle_rebuild_message(app: &mut App, msg: Msg, services: Option<&Services>) {
    match msg {
        Msg::StartRebuild => session::handle_start_rebuild(app, services),
        Msg::RebuildSessionCreated { jobs } => session::handle_session_created(app, &jobs, services),
        Msg::RebuildJobStarted { job_idx } => session::handle_job_started(app, job_idx),
        Msg::RebuildJobOutput {
            job_idx,
            chunk,
            stream,
        } => session::handle_job_output(app, job_idx, chunk, stream),
        Msg::RebuildJobFinished { job_idx, result } => {
            session::handle_job_finished(app, job_idx, result);
        }
        Msg::RebuildAdvance => session::handle_rebuild_advance(app, services),
        Msg::RebuildAborted(reason) => session::handle_rebuild_aborted(app, reason),
        Msg::RebuildAllDone => session::handle_rebuild_complete(app),
        Msg::OpenWorkQueue => work_queue::handle_open_work_queue(app),
        Msg::CloseModal => work_queue::handle_close_modal(app),
        Msg::WorkQueueUp => work_queue::handle_work_queue_up(app),
        Msg::WorkQueueDown => work_queue::handle_work_queue_down(app),
        Msg::WorkQueueSelect => work_queue::handle_work_queue_select(app),
        Msg::ToggleCheckAll => work_queue::handle_toggle_check_all(app),
        Msg::ScrollOutputUp
        | Msg::ScrollOutputDown
        | Msg::ScrollOutputPageUp
        | Msg::ScrollOutputPageDown
        | Msg::ScrollOutputTop
        | Msg::ScrollOutputBottom
        | Msg::ScrollOutputLeft
        | Msg::ScrollOutputRight => scroll::handle_scroll_message(app, &msg),
        Msg::OpenExportLog => export_log::handle_open_export_log(app),
        Msg::ExportInput(ch) => export_log::handle_export_input(app, ch),
        Msg::ExportBackspace => export_log::handle_export_backspace(app),
        Msg::ExportCancel => export_log::handle_export_cancel(app),
        Msg::ExportSubmit => export_log::handle_export_submit(app, services),
        Msg::StartSearchForward => search::handle_search_start(app, SearchDirection::Forward),
        Msg::StartSearchBackward => search::handle_search_start(app, SearchDirection::Backward),
        Msg::SearchInput(ch) => search::handle_search_input(app, ch),
        Msg::SearchBackspace => search::handle_search_backspace(app),
        Msg::SearchSubmit => search::handle_search_submit(app),
        Msg::SearchCancel => search::handle_search_cancel(app),
        Msg::SearchNext => search::handle_search_next(app),
        Msg::SearchPrev => search::handle_search_prev(app),
        Msg::ShowRebuild => session::handle_show_rebuild(app),
        Msg::ExitRebuild => session::handle_exit_rebuild(app),
        Msg::DockerfileNameUp => dockerfile_modal::handle_dockerfile_modal_move(app, -1),
        Msg::DockerfileNameDown => dockerfile_modal::handle_dockerfile_modal_move(app, 1),
        Msg::DockerfileNameInput(ch) => dockerfile_modal::handle_dockerfile_modal_input(app, ch),
        Msg::DockerfileNameLeft => dockerfile_modal::handle_dockerfile_modal_left(app),
        Msg::DockerfileNameRight => dockerfile_modal::handle_dockerfile_modal_right(app),
        Msg::DockerfileNameBackspace => dockerfile_modal::handle_dockerfile_modal_backspace(app),
        Msg::DockerfileNameAccept => {
            dockerfile_modal::handle_dockerfile_modal_accept(app, services);
        }
        Msg::DockerfileNameCancel => dockerfile_modal::handle_dockerfile_modal_cancel(app),
        _ => {}
    }
}

pub(super) fn queue_rebuild_jobs(
    app: &mut App,
    services: Option<&Services>,
    specs: Vec<RebuildJobSpec>,
) {
    if let Some(svc) = services {
        let start_idx = app.rebuild.as_ref().map_or(0, |state| state.jobs.len());
        session::handle_session_created(app, &specs, services);
        spawn_rebuild_thread(specs, svc, start_idx);
    }
}

pub(super) fn refresh_search_for_active_job(rebuild: &mut RebuildState) {
    search::refresh_search_for_active_job(rebuild);
}

pub(super) fn clamp_usize_to_u16(value: usize) -> u16 {
    scroll::clamp_usize_to_u16(value)
}

pub(super) fn default_line_buffer_limit(services: Option<&Services>) -> usize {
    services.map_or(REBUILD_VIEW_LINE_BUFFER_DEFAULT, |svc| {
        svc.args.rebuild_view_line_buffer_max
    })
}
